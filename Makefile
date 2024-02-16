dev:
	docker-compose up -d

dev-down:
	docker-compose down

generate-migration:
	cargo sqlx migrate add -r migration_name

migrate-up:
	cargo sqlx migrate run

migrate-down:
	cargo sqlx migrate revert

start-server:
	cargo watch -q -c -w src/ -x run

install:
	cargo add actix-web
	cargo add actix-cors
	cargo add serde_json
	cargo add serde --features derive
	cargo add chrono --features serde
	cargo add env_logger
	cargo add dotenv
	cargo add uuid --features "serde v4"
	cargo add sqlx --features "postgres chrono uuid"
	# HotReload
	cargo install cargo-watch
	# SQLX-CLI
	cargo install sqlx-cli

init-docker:
	sudo dockerd
	eval "$(docker-machine env default)"
	sudo docker-compose --verbose up -d
	export no_proxy=192.168.99.100
	sudo docker-compose up -d

database-docker:
	docker ps
	docker exec -it 5e529e923184 bash
	psql -h localhost -U admin -d backend

show-database-info:
	docker ps
	docker inspect postgres
