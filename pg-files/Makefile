CONNECT=host=10.1.1.3 user=postgres dbname=files
SCHEMA=v3

CARGO_FLAGS=--release
BIN_FLAGS=--connect="$(CONNECT)" --schema="$(SCHEMA)"
CMD=cargo run $(CARGO_FLAGS) -- $(BIN_FLAGS)

.PHONY: setup init updates

setup:
	$(CMD) setup

init: setup
	$(CMD) init_project --project 1 --dir input/node_modules_v1

updates: init
	$(CMD) update_project --project 1 --dir input/node_modules_v2 --diff input/diff_v1_v2.txt --version 2
	$(CMD) update_project --project 1 --dir input/node_modules_v3 --diff input/diff_v2_v3.txt --version 3

query:
	$(CMD) query --project 1 --mode list --version 2
