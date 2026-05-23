# Wdrożenie w Kubernetes

Dokument opisuje konfigurację wdrożenia aplikacji Sprouts w klastrze Kubernetes uruchomionym w Minikube. Uzupełnia manifesty z katalogu `k8s/` i zawiera uzasadnienia wymagane w zadaniu projektowym nr 2.

## Architektura

Sprouts składa się z trzech głównych komponentów:

- `frontend` - aplikacja SPA serwowana przez nginx,
- `backend` - API HTTP napisane w Rust/Axum,
- `database` - PostgreSQL przechowujący użytkowników, sesje, gry i ruchy.

Dodatkowo wdrożenie zawiera jednorazowy komponent `backend-migrate`, który wykonuje migracje bazy danych przed uruchomieniem replik backendu.

Przepływ ruchu:

```text
browser -> Ingress sprouts.local -> Service frontend -> frontend Pods
frontend nginx /api -> Service backend -> backend Pods
backend -> Service database -> PostgreSQL StatefulSet -> PVC
```

## Pliki manifestów

Manifesty znajdują się w katalogu `k8s/`:

- [`00-namespace.yaml`](../k8s/00-namespace.yaml) - dedykowany namespace `sprouts`,
- [`01-configmap.yaml`](../k8s/01-configmap.yaml) - konfiguracja jawna aplikacji i PostgreSQL,
- [`02-secret.yaml`](../k8s/02-secret.yaml) - hasło bazy danych oraz `DATABASE_URL`,
- [`03-serviceaccount.yaml`](../k8s/03-serviceaccount.yaml) - konto serwisowe dla podów aplikacji,
- [`05-networkpolicy.yaml`](../k8s/05-networkpolicy.yaml) - polityki sieciowe ograniczające ruch przychodzący,
- [`10-database.yaml`](../k8s/10-database.yaml) - Service i StatefulSet PostgreSQL,
- [`15-backend-migration-job.yaml`](../k8s/15-backend-migration-job.yaml) - Job uruchamiający migracje SQLx,
- [`20-backend.yaml`](../k8s/20-backend.yaml) - Service i Deployment backendu,
- [`30-frontend.yaml`](../k8s/30-frontend.yaml) - Service i Deployment frontendu,
- [`40-ingress.yaml`](../k8s/40-ingress.yaml) - Ingress HTTP dla `sprouts.local`.

## Namespace

Wszystkie obiekty aplikacji są wdrażane w dedykowanym namespace `sprouts`, zdefiniowanym w [`00-namespace.yaml`](../k8s/00-namespace.yaml). Oddziela to zasoby projektu od pozostałych obiektów klastra i ułatwia operacje administracyjne, np. podgląd zasobów lub usunięcie całego wdrożenia:

```bash
kubectl get all -n sprouts
kubectl delete namespace sprouts
```

## ConfigMap i Secret

Zgodnie z dobrymi praktykami zmienne konfiguracyjne nie są zaszyte w obrazach kontenerów. Wartości takie jak `APP_ENV`, `RUST_LOG`, `BIND_ADDRESS`, `DATABASE_MAX_CONNECTIONS`, `POSTGRES_USER` i `POSTGRES_DB` są przekazywane przez [`ConfigMap`](../k8s/01-configmap.yaml).
Dane wrażliwe, czyli `POSTGRES_PASSWORD` oraz `DATABASE_URL`, są przekazywane przez [`Secret`](../k8s/02-secret.yaml).

W Minikube `APP_ENV` jest ustawione na `development`, ponieważ wdrożenie używa lokalnego Ingress HTTP bez TLS. Dla wdrożenia produkcyjnego należałoby skonfigurować HTTPS i uruchomić aplikację w trybie produkcyjnym.

## PostgreSQL, StatefulSet i storage

PostgreSQL jest komponentem stanowym, dlatego został wdrożony jako `StatefulSet` w [`10-database.yaml`](../k8s/10-database.yaml). Baza ma jedną replikę, ponieważ projekt nie konfiguruje replikacji PostgreSQL. Dane są przechowywane w PVC tworzonym przez `volumeClaimTemplates`.

Konfiguracja trwałego przechowywania danych obejmuje:

- `StorageClass standard`, czyli domyślną klasę Minikube,
- PVC `database-data` o rozmiarze `1Gi`,
- tryb dostępu `ReadWriteOnce`.

Aplikacja deklaruje zapotrzebowanie na wolumen przez PVC, a Minikube przez domyślną klasę `standard` dynamicznie tworzy PV. Dane bazy nie są związane bezpośrednio z cyklem życia konkretnego poda.

## Migracje bazy danych

Migracje zostały wydzielone z procesu uruchamiania serwera HTTP. Obraz backendu obsługuje dwa tryby:

- `backend migrate` - wykonuje migracje SQLx i kończy proces,
- `backend serve` - uruchamia serwer HTTP.

W Kubernetes migracje wykonuje `Job` `backend-migrate`. Dzięki temu backend pozostaje bezstanowym mikroserwisem i może działać w wielu replikach bez ryzyka, że każda replika będzie próbowała wykonywać migracje przy starcie.

Surowe manifesty Kubernetes nie narzucają kolejności "baza -> migracje -> backend". Z tego powodu wdrożenie w Minikube jest wykonywane przez skrypt [`k8s/deploy-minikube.sh`](../k8s/deploy-minikube.sh), który stosuje manifesty fazami i używa `kubectl wait` do oczekiwania na gotowość bazy oraz zakończenie Job.

## Backend i frontend

Backend i frontend są wdrożone jako `Deployment`, ponieważ są bezstanowymi mikroserwisami aplikacyjnymi. Kubernetes może odtwarzać ich pody po awarii oraz wykonywać rolling update.

Backend jest zdefiniowany w [`20-backend.yaml`](../k8s/20-backend.yaml):

- obraz: `piasekdev/sprouts-backend:latest`,
- repliki: `2`,
- Service typu `ClusterIP` na porcie `3000`,
- readiness probe: `/api/readyz`,
- liveness probe: `/api/healthz`,
- limit puli połączeń do bazy: `DATABASE_MAX_CONNECTIONS=5` na replikę.

Frontend jest zdefiniowany w [`30-frontend.yaml`](../k8s/30-frontend.yaml):

- obraz: `piasekdev/sprouts-frontend:latest`,
- repliki: `2`,
- Service typu `ClusterIP` na porcie `80`,
- readiness/liveness probe na `/`.

Wykorzystanie kilku replik frontendu oraz backendu obrazuje skalowanie komponentów bezstanowych. Stan aplikacji jest przechowywany w PostgreSQL, więc restart lub podmiana podów nie usuwa danych aplikacji.

## Services i DNS

Wszystkie usługi aplikacji są typu `ClusterIP`. Nie są wystawiane bezpośrednio na zewnątrz klastra. Komunikacja między komponentami odbywa się przez stabilne nazwy DNS:

- `database`,
- `backend`,
- `frontend`.

Dzięki temu pody mogą być odtwarzane z nowymi adresami IP, a konfiguracja aplikacji nadal wskazuje na stabilną usługę.

## Ingress

Dostęp z zewnątrz klastra jest realizowany przez `Ingress` z `ingressClassName: nginx`, zdefiniowany w [`40-ingress.yaml`](../k8s/40-ingress.yaml). W Minikube wymaga to włączenia dodatku:

```bash
minikube addons enable ingress
```

Ingress kieruje ruch dla hosta `sprouts.local` do Service `frontend`. Został on wybrany, ponieważ wdrażany system jest aplikacją webową HTTP, a dostęp przez nazwę hosta dobrze odpowiada sposobowi korzystania z aplikacji w przeglądarce.

Wdrożenie nie wystawia backendu ani bazy danych bezpośrednio poza klaster. Usługi `frontend`, `backend` i `database` pozostają typu `ClusterIP`, a publiczny ruch HTTP przechodzi tylko przez Ingress Controller.

## Ograniczenia zasobów

Kontenery mają zdefiniowane `resources.requests` i `resources.limits`. Requests pomagają schedulerowi dobrać węzeł, a limits ograniczają maksymalne zużycie CPU i pamięci.

Przykładowe wartości:

- database: request `250m/256Mi`, limit `1 CPU/512Mi`,
- backend: request `250m/256Mi`, limit `1 CPU/768Mi`,
- frontend: request `100m/128Mi`, limit `500m/256Mi`,
- backend-migrate: request `100m/128Mi`, limit `500m/256Mi`.

## NetworkPolicy

Namespace ma domyślną politykę blokującą ruch przychodzący do podów. Następnie dodane są wyjątki:

- Ingress Controller może łączyć się z frontendem na porcie `80`,
- frontend może łączyć się z backendem na porcie `3000`,
- backend i `backend-migrate` mogą łączyć się z bazą na porcie `5432`.

Polityka sieciowa wymaga CNI wspierającego NetworkPolicy. Dla Minikube używany jest Calico:

```bash
minikube start --network-plugin=cni --cni=calico --container-runtime=docker
```

## Sterowanie rozmieszczeniem podów

Manifesty zawierają reguły affinity jako dodatkowy mechanizm sterowania planowaniem:

- backend preferuje uruchomienie blisko poda bazy danych,
- frontend preferuje rozproszenie replik po różnych węzłach.

Reguły są typu `preferredDuringSchedulingIgnoredDuringExecution`, a nie `required`. Dzięki temu manifesty działają także w jednowęzłowym Minikube, ale nadal pokazują intencje planowania w większym klastrze.

## ServiceAccount

Pody aplikacji używają dedykowanego [`ServiceAccount`](../k8s/03-serviceaccount.yaml) `sprouts-runtime` z `automountServiceAccountToken: false`. Kontenery nie potrzebują komunikować się z API Kubernetes, więc token konta serwisowego nie jest automatycznie montowany do podów.

Jest to element utwardzenia konfiguracji: frontend, backend i baza danych nie powinny mieć domyślnie dostępnego tokena pozwalającego na komunikację z API klastra. Gdyby któryś workload wymagał takiego dostępu, powinien otrzymać osobne konto serwisowe oraz wąskie uprawnienia RBAC.

## Procedura uruchomienia w Minikube

Środowisko:

```bash
minikube start --network-plugin=cni --cni=calico --container-runtime=docker
minikube addons enable ingress
kubectl get nodes
kubectl get pods -A
```

Wdrożenie aplikacji:

```bash
bash k8s/deploy-minikube.sh
```

Skrypt wykonuje:

1. sprawdzenie dostępności klasy storage `standard`,
2. utworzenie namespace, konfiguracji, sekretów, konta serwisowego i NetworkPolicy,
3. wdrożenie PostgreSQL i oczekiwanie na gotowość StatefulSet,
4. uruchomienie Job `backend-migrate` i oczekiwanie na jego zakończenie,
5. wdrożenie backendu, frontendu i Ingress,
6. oczekiwanie na gotowość Deploymentów,
7. wypisanie najważniejszych zasobów.

Po wdrożeniu należy dodać lokalny wpis DNS:

```bash
echo "$(minikube ip) sprouts.local" | sudo tee -a /etc/hosts
```

Podstawowa weryfikacja:

```bash
kubectl get all -n sprouts
kubectl get ingress,pvc,networkpolicy -n sprouts
kubectl get job backend-migrate -n sprouts
curl -i http://sprouts.local/api/healthz
curl -i http://sprouts.local/api/readyz
curl -I http://sprouts.local
```
