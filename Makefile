all: compile

compile:
	cargo build --release

USER :=  $(shell whoami)
PREFIX := "/usr/"

install: compile
	@echo
	@echo "Installing to ${PREFIX}"
	@echo
	mkdir -p ${PREFIX}/../etc/systemd/system/
	mkdir -p ${PREFIX}/local/bin/
	sudo cp aseqkeeper.service ${PREFIX}/../etc/systemd/system/
	# Sorry it can not be nobody. Need to store the status somewhere.
	sudo sed -i s/nobody/${USER}/g ${PREFIX}/../etc/systemd/system/aseqkeeper.service
	sudo cp target/release/aseqkeeper ${PREFIX}/local/bin/
	sudo systemctl daemon-reload
	@echo
	@echo "Enable for next reboot with: sudo systemctl enable aseqkeeper"
	@echo
	@echo "Start it now with: sudo service aseqkeeper start"
	@echo
