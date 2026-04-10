# Sprouts

Projekt `Sprouts` jest serwisem do gry multiplayer online opartym na architekturze trzech kontenerów:

- `frontend` - aplikacja SPA zbudowana w `Leptos` i kompilowana do WebAssembly,
- `backend` - serwer HTTP w `Rust` z wykorzystaniem `Axum`, odpowiedzialny za logikę gry, autoryzację i walidację ruchów,
- `database` - baza danych `PostgreSQL`, przechowująca użytkowników, sesje, gry i historię ruchów.

Najważniejsze katalogi repozytorium:

- `backend/` - kod backendu i `backend/Dockerfile`
- `frontend/` - kod frontendu i `frontend/Dockerfile`
- `shared/` - współdzielone typy request/response oraz model stanu planszy
- `migrations/` - migracje bazy danych
- `.sqlx/` - metadane SQLx do buildów offline
- `docs/` - dokumentacja do zadania cząstkowego nr 1

Repozytorium zawiera również dokumentację przygotowaną pod zadanie cząstkowe nr 1 z przedmiotu `Bezpieczeństwo procesów CI/CD`.

## Autor

Maciej Piasecki
Nr indeksu: 97701
Grupa: 1

## Główne funkcje projektu

- rejestracja i logowanie użytkowników,
- utrzymywanie sesji użytkownika w ciasteczku HTTP-only,
- tworzenie nowej gry,
- dołączanie do gry przez 8-znakowy kod pokoju,
- pobieranie aktualnego stanu planszy,
- wysyłanie ruchów i walidacja zasad gry po stronie backendu.

## Architektura i przepływ danych

Frontend jest serwowany przez `nginx` i komunikuje się z backendem przez ścieżkę `/api`, która jest przekazywana przez reverse proxy do usługi `backend`.
Backend udostępnia endpointy `auth` i `game`, a cała trwała warstwa danych znajduje się w PostgreSQL.
Aktualny stan planszy gry jest przechowywany w kolumnie `JSONB`, a historia wykonanych ruchów zapisywana jest w osobnej tabeli `moves`.

## Pliki kontenerowe

W repozytorium znajdują się:

- `frontend/Dockerfile`
- `backend/Dockerfile`
- `docker-compose.yaml`
- `docker-compose.prod.yaml`

## Uruchomienie lokalne

Najprostszy wariant:

```bash
docker compose up --build
```

Wariant produkcyjny oparty o opublikowane obrazy:

```bash
docker compose -f docker-compose.yaml -f docker-compose.prod.yaml up -d
```

Plik `docker-compose.prod.yaml` stanowi wariant produkcyjny używany razem z bazową konfiguracją z `docker-compose.yaml`.

Po uruchomieniu:

- frontend: <http://localhost:8080>
- backend: <http://localhost:3000>
- backend API: <http://localhost:3000/api>
- PostgreSQL: `localhost:5432`

Zatrzymanie środowiska:

```bash
docker compose down
```

Usunięcie środowiska wraz z wolumenem bazy danych:

```bash
docker compose down -v
```

## SQLx offline metadata

Backend wykorzystuje zapytania `SQLx` sprawdzane na etapie kompilacji.
W obrazie backendu ustawiono zmienną:

```bash
SQLX_OFFLINE=true
```

Dlatego build obrazu backendu nie wymaga działającej bazy danych w trakcie kompilacji, ale wymaga aktualnych metadanych w katalogu `.sqlx/`.

Aby odświeżyć metadane lokalnie przy działającej bazie danych:

```bash
DATABASE_URL=postgres://admin:admin@localhost:5432/sprouts cargo sqlx prepare --workspace
```

Aby sprawdzić, czy metadane są aktualne:

```bash
DATABASE_URL=postgres://admin:admin@localhost:5432/sprouts cargo sqlx prepare --check --workspace
```

## Dokumentacja do zadania cząstkowego nr 1

Szczegółowe opisy poszczególnych punktów zadania znajdują się w poniższych plikach:

- [Punkt 2 - Dockerfile dla mikroserwisów](docs/02-dockerfiles.md)
- [Punkt 3 - Budowanie obrazów, publikacja i SBOM](docs/03-build-push-sbom.md)
- [Punkt 4 - Analiza podatności](docs/04-vulnerability-scan.md)
- [Punkt 5 - Testowa wersja docker-compose](docs/05-docker-compose.md)
- [Punkt 6 - Diagram compose-viz](docs/06-compose-viz.md)

## Repozytorium źródłowe

- GitHub: <https://github.com/PiasekDev/sprouts>
- DockerHub backend: <https://hub.docker.com/r/piasekdev/sprouts-backend>
- DockerHub frontend: <https://hub.docker.com/r/piasekdev/sprouts-frontend>
