TMP_FILE := $(shell mktemp)

all: use_case_tests

use_case_tests: mv_files

mv_files: mv_files.expected
	@echo "+ $@"
	@cd ..; cargo run -q --bin mv_files -- --no-color tests/data /tmp --size 0 -d | grep -v '^$@' | sort > $(TMP_FILE)
	@diff $< $(TMP_FILE)
	@echo "+ $@ okay"

