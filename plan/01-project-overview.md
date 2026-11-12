# Orchestrator — Project Overview

## Problem Statement

Modern applications need fine-grained traffic control — routing percentages, canary deployments, blue-green rollouts, multi-datacenter failover, DNS-based routing. Existing solutions (F5, commercial load balancers) are:

- **Proprietary and expensive** — hardware appliances with vendor lock-in
- **Complex to automate** — closed config languages, heavy management planes
- **Not designed for modern apps** — built for monolithic datacenters, not distributed microservices
- **No DNS-level traffic routing** — L4/L7 proxy only, DNS is a separate product
- **Slow to provision** — days or weeks to set up new routing rules

## Solution

**Orchestrator** is an open-source, application-level traffic manager that:

- Routes traffic across regions, data centers, clusters, and endpoints with configurable weights
- Manages HA worker pairs with Linux kernel VIP failover
- Handles blue-green and canary rolling updates with auto-rollback on error thresholds
- Serves authoritative DNS for tenant zones
- Terminates TLS at the VIP level
- Runs per-tenant as an isolated process
- Provides a web GUI for configuration and visualization

## Key Differentiators vs F5

| F5 | Orchestrator |
|----|-------------|
| Hardware appliance, expensive | Commodity hardware, open source |
| Proprietary config language | YAML (human-readable, version-controllable) |
| Heavy centralized management | Lightweight YAML + GUI |
| L4/L7 proxy only | L4/L7 proxy + DNS-level routing |
| Monolithic binary | Per-tenant process isolation |
| Slow to provision | Spin up a tenant in seconds |
| Vendor lock-in | Open source, community-driven |
| Complex automation | GitOps-friendly, hot-reloadable |

## What It Does

1. **Routes traffic** to Kubernetes clusters and VM pools with weighted splits
2. **Manages HA worker pairs** with VIP failover via Linux kernel iproute2
3. **Handles blue-green + canary rolling updates** with auto-rollback on error thresholds
4. **Serves authoritative DNS** for tenant zones (basic A/SRV records, forwards everything else upstream)
5. **Terminates TLS** at the VIP level

## What It Does Not Do (Yet)

- Full DNS server (forwarding handled upstream)
- Advanced metrics (stubbed, Prometheus-ready, but not first priority)
- RBAC for GUI (username/password only, auth enhancements later)
- Kubernetes dependency (this is standalone)
