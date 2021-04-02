.PHONY: install-npm-packages build-rust build-chunks build-server build-client run-server run-client

PROJECT=io-test

install-npm-packages:
	npm install

build-rust:
	cd fs-rebuild \
		&& cargo build --release
	cp fs-rebuild/target/release/fs-rebuild client/fs-rebuild

build-server: install-npm-packages
	docker build -f server/Containerfile -t "$(PROJECT):server" --net host

build-client: build-rust
	docker build -f client/Containerfile -t "$(PROJECT):client" --net host

run-server:
	docker run --name "$(PROJECT)-server" \
		--rm --net host \
		--volume "./node_modules:/mnt/data:z" \
		-it "localhost/$(PROJECT):server"

run-client:
	docker run --name "$(PROJECT)-client" \
		--rm --net host \
		-it "localhost/$(PROJECT):client"
