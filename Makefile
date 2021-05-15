PROJECT=io-test

# Utility
.PHONY: install-npm-packages build-rust clear-containerd

install-npm-packages:
	npm install

build-rust:
	cd fs-rebuild \
		&& cargo build --release
	cp fs-rebuild/target/release/fs-rebuild client/fs-rebuild

clear-containerd:
	sudo ctr t ls -q | xargs --no-run-if-empty sudo ctr t kill -s 9
	sudo ctr c ls -q | xargs --no-run-if-empty sudo ctr c rm

# Build Server
.PHONY: build-server build-server-docker build-server-containerd

build-server: install-npm-packages
	sudo podman build -f server/Containerfile -t "$(PROJECT):server" --net host

build-server-docker: install-npm-packages
	sudo docker build -t "$(PROJECT):server" -f ./server/Containerfile ./server

build-server-containerd: build-server-docker
	sudo docker save -o ./server.tar "$(PROJECT):server"
	sudo ctr images import ./server.tar

# Build Client
.PHONY: build-client build-client-docker build-client-containerd

build-client: build-rust
	sudo podman build -f client/Containerfile -t "$(PROJECT):client" --net host

build-client-docker: build-rust
	sudo docker build -t "$(PROJECT):client" -f ./client/Containerfile ./client

build-client-containerd: build-client-docker
	sudo docker save -o ./client.tar "$(PROJECT):client"
	sudo ctr images import ./client.tar

# Run Server
.PHONY: run-server run-server-gvisor run-server-kata run-server-kata-v2 run-server-containerd

run-server:
	sudo podman run --rm --name "$(PROJECT)-server" \
		--net host \
		--volume "./node_modules:/mnt/data:z" \
		-it "localhost/$(PROJECT):server"

run-server-gvisor:
	sudo docker run --rm --name "$(PROJECT)-server" \
		--volume "$(shell pwd)/node_modules:/mnt/data" \
		-it "$(PROJECT):server"

run-server-kata:
	sudo docker run --rm --name "$(PROJECT)-server" \
		--runtime kata-runtime \
		--volume "$(shell pwd)/node_modules:/mnt/data" \
		-it "$(PROJECT):server"

run-server-kata-v2:
	sudo nerdctl run --rm --name "$(PROJECT)-server" \
		--hostname "$(PROJECT)-server" \
		--volume "$(shell pwd)/node_modules:/mnt/data" \
		-it "$(PROJECT):server"

run-server-containerd:
	sudo ctr run --rm \
		--tty \
		--runtime "io.containerd.kata.v2" \
		--mount "type=bind,src=$(shell pwd)/node_modules,dst=/mnt/data,options=rbind:ro" \
		"docker.io/library/$(PROJECT):server" server

# Run Client
.PHONY: run-client run-client-gvisor run-client-kata run-client-kata-v2 run-client-containerd

run-client:
	sudo podman run --rm --name "$(PROJECT)-client" \
		--net host \
		--mount "type=tmpfs,destination=/mnt/data,tmpfs-mode=1777,tmpfs-size=500000000" \
		-it "localhost/$(PROJECT):client"

run-client-gvisor:
	sudo docker run --rm --name "$(PROJECT)-client" \
		--runtime runsc \
		--link "$(PROJECT)-server:$(PROJECT)-server" \
		--mount "type=tmpfs,destination=/mnt/data,tmpfs-mode=1777,tmpfs-size=500000000" \
		-it "$(PROJECT):client"

run-client-kata:
	sudo docker run --rm --name "$(PROJECT)-client" \
		--runtime kata-runtime \
		--link "$(PROJECT)-server:$(PROJECT)-server" \
		--mount "type=tmpfs,destination=/mnt/data,tmpfs-mode=1777,tmpfs-size=500000000" \
		-it "$(PROJECT):client"

run-client-kata-v2:
	sudo nerdctl run --rm --name "$(PROJECT)-client" \
		--network mynet \
		-it "$(PROJECT):client"

run-client-containerd:
	sudo ctr run --rm \
		--tty \
		--runtime "io.containerd.kata.v2" \
		--mount "type=tmpfs,destination=/mnt/data" \
		"docker.io/library/$(PROJECT):client" client

# Build & Run
.PHONY: server server-gvisor server-kata server-kata-v2 server-containerd

server: build-server run-server

server-gvisor: build-server-docker run-server-gvisor

server-kata: build-server-docker run-server-kata

server-kata-v2: build-server-docker run-server-kata-v2

server-containerd: build-server-containerd run-server-containerd

.PHONY: client client-gvisor client-kata client-kata-v2 client-containerd

client: build-client run-client

client-gvisor: build-client-docker run-client-gvisor

client-kata: build-client-docker run-client-kata

client-kata-v2: build-client-containerd run-client-kata-v2

client-containerd: build-client-containerd run-client-containerd

# K8S
.PHONY: add-to-k8s clear-k8s k8s-server k8s-client

add-to-k8s: build-server build-client
	sudo podman save -o ./server.tar --format oci-archive "$(PROJECT):server"
	sudo podman save -o ./client.tar --format oci-archive "$(PROJECT):client"
	sudo ctr -n k8s.io images import ./server.tar
	sudo ctr -n k8s.io images import ./client.tar

clear-k8s:
	kubectl delete --all service
	kubectl delete --all pod

k8s-server:
	kubectl apply -f k8s/server-pod.yaml
	kubectl apply -f k8s/server-service.yaml

k8s-client:
	kubectl apply -f k8s/client-pod.yaml
	sleep 5
	kubectl exec -it client -- /bin/bash

k8s-client-trusted:
	kubectl apply -f k8s/client-trusted-pod.yaml
	sleep 5
	kubectl exec -it client-trusted -- /bin/bash
