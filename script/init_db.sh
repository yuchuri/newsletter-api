#!/usr/bin/env bash

# Fail script if any command fails or any pipeline component fails
set -eo pipefail

# Ensure psql is installed before proceeding
if ! [ -x "$(command -v psql)" ]; then
  echo >&2 "Error: psql is not installed."
  exit 1
fi

# Ensure sqlx-cli is installed before proceeding
if ! [ -x "$(command -v sqlx)" ]; then
  echo >&2 "Error: sqlx is not installed."
  echo >&2 "Use:"
  echo >&2 "cargo install sqlx-cli --no-default-features --features postgres"
  echo >&2 "to install it."
  exit 1
fi

# Check if custom environment variables are set; otherwise use defaults
DB_USER=${POSTGRES_USER:=postgres}
DB_PASSWORD=${POSTGRES_PASSWORD:=password}
DB_NAME=${POSTGRES_DB:=newsletter}
DB_PORT=${POSTGRES_PORT:=5432}

# Allow skipping Docker container launch if an instance is already running
if [[ -z "${SKIP_DOCKER}" ]]; then
  docker run \
    --name postgres \
    -e POSTGRES_USER=${DB_USER} \
    -e POSTGRES_PASSWORD=${DB_PASSWORD} \
    -e POSTGRES_DB=${DB_NAME} \
    -p "${DB_PORT}":5432 \
    -d \
    postgres -N 1000
    # ^ Increased maximum number of connections for testing purposes
fi

# Keep pinging PostgreSQL via psql until it is ready to accept connections
export PGPASSWORD="${DB_PASSWORD}"
until psql -h "localhost" -U "${DB_USER}" -p "${DB_PORT}" -d "postgres" -c '\q'; do
  >&2 echo "Postgres is still unavailable - sleeping"
  sleep 1
done

>&2 echo "Postgres is up and running on port ${DB_PORT}!"

# Set DATABASE_URL required by sqlx-cli
export DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@localhost:${DB_PORT}/${DB_NAME}

# Create the database if it doesn't already exist
sqlx database create

# Apply all database migrations from migrations/
sqlx migrate run

>&2 echo "Postgres has been migrated, ready to go!"