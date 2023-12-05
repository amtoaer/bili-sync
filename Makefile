.PHONY: install fmt start-daemon start-once db-init db-migrate db-upgrade sync-conf

install:
	@echo "Installing dependencies..."
	@poetry install --no-root

fmt:
	@echo "Formatting..."
	@poetry run black .
	@poetry run ruff --fix .

start-daemon:
	@poetry run python entry.py

start-once:
	@poetry run python entry.py --once

db-init:
	@poetry run aerich init-db

db-migrate:
	@poetry run aerich migrate

db-upgrade:
	@poetry run aerich upgrade

sync-conf:
	@echo "Syncing config..."
	@cp ${CONFIG_SRC} ./config/
	@cp ${DB_SRC} ./data/
	@echo "Done."