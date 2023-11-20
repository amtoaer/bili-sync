.PHONY: install lint

install:
	@echo "Installing dependencies..."
	@poetry install --no-root

fmt:
	@echo "Formatting..."
	@poetry run black .
	@poetry run ruff --fix .
