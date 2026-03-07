#!/bin/sh
mkdir -p /app/data
touch /app/data/chargeapi.db
exec "$@"
