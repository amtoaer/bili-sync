.PHONY: install fmt start-daemon start-once

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