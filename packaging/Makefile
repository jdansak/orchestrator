# Orchestrator — packaging
#
# The release pipeline calls this. It produces a DEB and an RPM from the same binaries.

VERSION ?= 0.1.0
ARCH ?= amd64
ROOT := $(shell cd .. && pwd)
BUILD := $(ROOT)/target/release
STAGE := $(CURDIR)/stage

.PHONY: all deb rpm clean

all: deb rpm

deb: ## Build the Debian package
	rm -rf $(STAGE)
	mkdir -p $(STAGE)/DEBIAN
	mkdir -p $(STAGE)/usr/local/bin
	mkdir -p $(STAGE)/lib/systemd/system
	mkdir -p $(STAGE)/etc/orchestrator/tenants
	mkdir -p $(STAGE)/usr/share/doc/orchestrator
	install -m 0755 $(BUILD)/orchestrator $(STAGE)/usr/local/bin/
	install -m 0755 $(BUILD)/orchestrator-manager $(STAGE)/usr/local/bin/
	install -m 0644 $(ROOT)/systemd/orchestrator-manager.service $(STAGE)/lib/systemd/system/
	install -m 0644 $(ROOT)/LICENSE $(ROOT)/README.md $(STAGE)/usr/share/doc/orchestrator/
	cp $(ROOT)/packaging/deb/control $(STAGE)/DEBIAN/control
	sed -i "s/^Version:.*/Version: $(VERSION)/" $(STAGE)/DEBIAN/control
	install -m 0755 $(ROOT)/packaging/deb/postinst $(STAGE)/DEBIAN/postinst
	install -m 0755 $(ROOT)/packaging/deb/postrm $(STAGE)/DEBIAN/postrm
	dpkg-deb --build --root-owner-group $(STAGE) $(CURDIR)/orchestrator_$(VERSION)_$(ARCH).deb

rpm: ## Build the RPM package
	cd $(ROOT) && rpmbuild -ta --define '_version $(VERSION)' $(CURDIR)/rpm/orchestrator.spec

clean:
	rm -rf $(STAGE) $(CURDIR)/*.deb
