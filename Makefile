.PHONY: install-npm-packages build-rust build-chunks build-server build-client run-server run-client

PROJECT=io-test

install-npm-packages:
	npm install

build-rust:
	cd fs-rebuild \
		&& cargo build --release
	cp fs-rebuild/target/release/fs-rebuild client/fs-rebuild

build-server: install-npm-packages
	sudo podman build -f server/Containerfile -t "$(PROJECT):server" --net host

build-server-docker: install-npm-packages
	sudo docker build -t "$(PROJECT):server" -f ./server/Containerfile ./server

build-client: build-rust
	sudo podman build -f client/Containerfile -t "$(PROJECT):client" --net host

build-client-docker: build-rust
	sudo docker build -t "$(PROJECT):client" -f ./client/Containerfile ./client

run-server:
	sudo podman run --name "$(PROJECT)-server" \
		--rm --net host \
		--volume "./node_modules:/mnt/data:z" \
		-it "localhost/$(PROJECT):server"

run-server-docker:
	sudo docker run --name "$(PROJECT)-server" \
		--rm \
		--volume "$(shell pwd)/node_modules:/mnt/data" \
		-it "$(PROJECT):server"

run-client:
	sudo podman run --name "$(PROJECT)-client" \
		--rm --net host \
		--volume "./example:/mnt/data:z" \
		-it "localhost/$(PROJECT):client"

run-client-docker:
	sudo docker run --name "$(PROJECT)-client" \
		--runtime runsc \
		--link "$(PROJECT)-server:$(PROJECT)-server" \
		--rm \
		--volume "$(shell pwd)/example:/mnt/data" \
		-it "$(PROJECT):client"
