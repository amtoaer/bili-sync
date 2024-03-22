migrate:
	@echo "Migrating database"
	@sea-orm-cli migrate up
	@sea-orm-cli generate entity -o entity/src/entities
	@echo "Database migrated"