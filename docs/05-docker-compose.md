# Punkt 5 - Testowa wersja docker-compose.yaml

W projekcie przygotowano plik `docker-compose.yaml` jako testową/deweloperską konfigurację środowiska oraz dodatkowy plik `docker-compose.prod.yaml` jako wariant produkcyjny oparty o obrazy opublikowane na DockerHub.

Podstawowy plik uruchamia cztery usługi:

- `database`
- `backend-migrate`
- `backend`
- `frontend`

## Konfiguracja usług

### database

- obraz: `postgres:18.3-trixie`
- konfiguracja ładowana z pliku `.env`
- port hosta: `5432`
- zmienne środowiskowe:
  - `POSTGRES_USER=admin`
  - `POSTGRES_PASSWORD=admin`
  - `POSTGRES_DB=sprouts`
- trwały wolumen: `database-data`
- healthcheck oparty o `pg_isready`
- sieć: `backend`
- limit zasobów CPU/RAM zdefiniowany w Compose

### backend-migrate

- build z `backend/Dockerfile`
- jednorazowe polecenie: `backend migrate`
- zależność od gotowości bazy danych przez `depends_on` i `condition: service_healthy`
- zmienne środowiskowe:
  - `DATABASE_URL=postgres://admin:admin@database:5432/sprouts`
  - `RUST_LOG=info`
- sieć: `backend`
- zadanie kończy działanie po poprawnym wykonaniu migracji SQLx

### backend

- build z `backend/Dockerfile`
- polecenie: `backend serve`
- konfiguracja ładowana z pliku `.env`
- port hosta: `3000`
- zależność od poprawnego zakończenia migracji przez `depends_on` i `condition: service_completed_successfully`
- zmienne środowiskowe:
  - `APP_ENV=development`
  - `BIND_ADDRESS=0.0.0.0:3000`
  - `DATABASE_MAX_CONNECTIONS=10`
  - `DATABASE_URL=postgres://admin:admin@database:5432/sprouts`
  - `RUST_LOG=info`
- sieci: `frontend`, `backend`
- limit zasobów CPU/RAM zdefiniowany w Compose

### frontend

- build z `frontend/Dockerfile`
- port hosta: `8080`
- zależność od backendu
- serwowanie SPA przez `nginx`
- reverse proxy z `/api` do `backend:3000`
- sieć: `frontend`
- limit zasobów CPU/RAM zdefiniowany w Compose

### debug-shell

- opcjonalna usługa pomocnicza uruchamiana tylko z profilem `debug`
- obraz: `busybox:1.36`
- przeznaczenie: szybka diagnostyka sieci i środowiska uruchomieniowego
- sieci: `frontend`, `backend`

## Uruchomienie

```bash
docker compose up --build
```

Uruchomienie z profilem debug:

```bash
docker compose --profile debug up --build
```

Wariant produkcyjny z wykorzystaniem opublikowanych obrazów:

```bash
docker compose -f docker-compose.yaml -f docker-compose.prod.yaml up -d
```

Plik `docker-compose.prod.yaml` nie jest samodzielną pełną definicją środowiska.
Pełni rolę pliku nadpisującego, który działa razem z `docker-compose.yaml` i podmienia tylko elementy specyficzne dla wariantu produkcyjnego, takie jak użycie opublikowanych obrazów zamiast lokalnego `build` oraz brak publicznego portu dla bazy danych.
Dotyczy to również usługi `backend-migrate`, która w wariancie produkcyjnym korzysta z tego samego opublikowanego obrazu backendu.

## Zastosowane dobre praktyki

Zastosowane dobre praktyki obejmują:

- jawne wersjonowanie pliku Compose (`version: "3.9"`),
- czytelny podział usług według odpowiedzialności,
- wydzielenie trwałego wolumenu dla danych,
- wykorzystanie pliku `.env` do konfiguracji środowiska,
- jawnie zdefiniowane sieci `frontend` i `backend`,
- zastosowanie `healthcheck` dla bazy danych,
- użycie `depends_on` dla kolejności uruchamiania oraz osobnej jednorazowej usługi migracyjnej,
- jawne mapowanie portów w środowisku deweloperskim,
- limity CPU/RAM dla usług,
- profil `debug` dla usługi pomocniczej,
- osobny plik `docker-compose.prod.yaml` dla wariantu produkcyjnego,
- zachowanie prostego i łatwego do utrzymania układu pliku.

## Wnioski

`docker-compose.yaml` pozwala uruchomić pierwszą działającą wersję aplikacji i potwierdzić poprawność integracji trzech głównych warstw systemu, natomiast `docker-compose.prod.yaml` rozdziela konfigurację produkcyjną od deweloperskiej zgodnie z zaleceniami.
