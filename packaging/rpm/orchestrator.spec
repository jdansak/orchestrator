Name:           orchestrator
Version:        0.1.0
Release:        1%{?dist}
Summary:        Application-level traffic manager for Kubernetes clusters and VM pools

License:        Apache-2.0
URL:            https://github.com/Jdansak/orchestrator
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  cargo >= 1.99
BuildRequires:  rust >= 1.99
Requires:       iproute
Requires:       systemd

%description
Orchestrator routes traffic across regions, data centres, clusters, and endpoints with
configurable weights. It manages HA worker pairs with Linux kernel VIP failover, handles
blue-green and canary rollouts with automatic rollback, serves authoritative DNS for tenant
zones, and terminates TLS at the VIP.

Each tenant runs as an isolated systemd service with its own configuration, VIP, listeners,
and resource limits.

%prep
%autosetup

%build
cargo build --workspace --release

%install
mkdir -p %{buildroot}%{_bindir}
install -m 0755 target/release/orchestrator %{buildroot}%{_bindir}/orchestrator
install -m 0755 target/release/orchestrator-manager %{buildroot}%{_bindir}/orchestrator-manager

mkdir -p %{buildroot}%{_unitdir}
install -m 0644 systemd/orchestrator-manager.service %{buildroot}%{_unitdir}/

mkdir -p %{buildroot}%{_sysconfdir}/orchestrator/tenants
mkdir -p %{buildroot}%{_localstatedir}/lib/orchestrator
mkdir -p %{buildroot}%{_localstatedir}/log/orchestrator

%pre
getent group orchestrator >/dev/null || groupadd --system orchestrator
getent passwd orchestrator >/dev/null || \
  useradd --system --gid orchestrator --no-create-home \
    --home-dir %{_localstatedir}/lib/orchestrator --shell /sbin/nologin orchestrator
exit 0

%post
%systemd_post orchestrator-manager.service

%preun
%systemd_preun orchestrator-manager.service

%postun
%systemd_postun_with_restart orchestrator-manager.service

%files
%{_bindir}/orchestrator
%{_bindir}/orchestrator-manager
%{_unitdir}/orchestrator-manager.service
%config(noreplace) %attr(0700,orchestrator,orchestrator) %{_sysconfdir}/orchestrator
%attr(0750,orchestrator,orchestrator) %dir %{_localstatedir}/lib/orchestrator
%attr(0750,orchestrator,orchestrator) %dir %{_localstatedir}/log/orchestrator
%doc LICENSE
%doc README.md
%doc plan/*.md

%changelog
* Thu Nov 12 2026 Orchestrator contributors <maintainers@orchestrator.dev> - 0.1.0-1
- Initial packaging
