.PHONY: help clean


debug:=
verbose:=
unit-tests:=unit-tests-passed.txt
cargo.target_dir=target

arg.demo_bin=beholder
arg.test_filename=softdirty-repro.mmap
arg.tmpfs_path:=/dev/shm
arg.hugetlbfs_path:=/mnt/huge

arg.page_size.normal=normal
arg.page_size.huge=huge
arg.page_count=3
arg.loops=3
arg.assert_behavior=panic


rust:=$(HOME)/.cargo


all: test


$(rust):
	@echo "Installing rust..."
	curl https://sh.rustup.rs -sSf | sh -s -- -y


$(arg.hugetlbfs_path):
	@echo "Mounting Hugetlbfs to $(arg.hugetlbfs_path)..."
	./mount-hugetlbfs.sh


$(cargo.target_dir):
	@echo "Building..."
	cargo build $(verbose)


build: $(rust) $(cargo.target_dir)


$(unit-tests): $(rust) build
	@echo "Running unit tests..."
	cargo test $(verbose) --color=always 2>&1 | tee  $(unit-tests)


help: $(rust) build
	@cargo run --bin $(arg.demo_bin) -- --help


run-tmpfs-test: $(rust) build $(arg.tempfs_path)
	@echo "Running test with $(arg.page_size.normal) pages and $(arg.tmpfs_path)"
	cargo run --bin $(arg.demo_bin) -- \
		$(debug) \
		$(verbose) \
		demo \
		--path $(arg.tmpfs_path)/$(arg.test_filename) \
		--page-size=$(arg.page_size.normal) \
		--page-count=$(arg.page_count) \
		--loops=$(arg.loops) \
		--assert=$(arg.assert_behavior) \
	&& echo "TEST SUCCESS" || echo "TEST FAILED"


run-hugetlbfs-test: $(rust) build $(arg.hugetlbfs_path)
	@echo "Running test with $(arg.page_size.huge) pages and $(arg.hugetlbfs_path)"
	cargo run --bin $(arg.demo_bin) -- \
		$(debug) \
		$(verbose) \
		demo \
		--path $(arg.hugetlbfs_path)/$(arg.test_filename) \
		--page-size=$(arg.page_size.huge) \
		--page-count=$(arg.page_count) \
		--loops=$(arg.loops) \
		--assert=$(arg.assert_behavior)  \
	&& echo "TEST SUCCESS" || echo "TEST FAILED"



test: $(rust) build $(unit-tests) run-tmpfs-test run-hugetlbfs-test



clean:
	@echo "Cleaning..."
	cargo clean $(verbose)
	-rm -fv *~
	-rm -rfv $(unit-tests)
	-rm -rfv $(cargo.target_dir)
	-rm -fv $(arg.hugetlbfs_path)/$(arg.test_filename)
	-rm -fv $(arg.tmpfs_path)/$(arg.test_filename)
	-sudo umount $(arg.hugetlbfs_path) 

