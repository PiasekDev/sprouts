# Punkt 2 - Dockerfile dla mikroserwisów

W projekcie przygotowano dwa pliki `Dockerfile`:

- `frontend/Dockerfile`
- `backend/Dockerfile`

Usługa bazy danych nie korzysta z własnego obrazu. W `docker-compose.yaml` wykorzystywany jest oficjalny obraz `postgres:18.3-trixie`.

## Backend

Plik:

- `backend/Dockerfile`

Charakterystyka:

- zastosowano `multi-stage build`,
- etap budowania korzysta z obrazu `rust:1.94-trixie`,
- binarka `backend` jest kopiowana do lekkiego obrazu runtime `debian:trixie-slim`,
- w obrazie runtime instalowane są tylko niezbędne certyfikaty CA,
- aplikacja działa jako użytkownik nieuprzywilejowany `sprouts`,
- kontener udostępnia port `3000`.

Dobre praktyki zastosowane w pliku:

- rozdzielenie warstwy build i runtime,
- ograniczenie liczby pakietów w obrazie końcowym,
- uruchamianie procesu jako `non-root`,
- jawne ustawienie katalogu roboczego i portu,
- wykorzystanie pliku `.dockerignore` na poziomie repozytorium.

## Frontend

Plik:

- `frontend/Dockerfile`

Charakterystyka:

- zastosowano `multi-stage build`,
- etap budowania również korzysta z obrazu `rust:1.94-trixie`,
- w etapie build instalowane są `trunk` i target `wasm32-unknown-unknown`,
- aplikacja jest kompilowana do statycznych plików frontendowych,
- gotowe pliki trafiają do obrazu runtime `nginx:1.29.6-alpine3.23`,
- kontener udostępnia port `80`.

Dobre praktyki zastosowane w pliku:

- lekki obraz runtime oparty o `nginx:alpine`,
- oddzielenie procesu budowania WASM od serwowania statycznych plików,
- brak toolchaina Rust w obrazie końcowym,
- dedykowana konfiguracja `nginx`, która obsługuje zarówno SPA, jak i reverse proxy do backendu.

## .dockerignore

Na poziomie repozytorium wykorzystano plik `.dockerignore`, który pomija m.in.:

- `.git`,
- `target`,
- `frontend/dist`,
- pliki logów,
- lokalne pliki `.env`.

Zmniejsza to kontekst budowania i ogranicza kopiowanie niepotrzebnych plików do procesu build.

## Wnioski

Przygotowane pliki `Dockerfile` realizują podstawowe dobre praktyki budowania obrazów kontenerowych:
ograniczają rozmiar obrazów końcowych, rozdzielają etapy budowania od runtime i minimalizują powierzchnię ataku.
