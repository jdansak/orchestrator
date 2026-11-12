/**
 * The API endpoint wrappers.
 *
 * One function per endpoint, so components never build URLs by hand.
 */

import { request } from './client';

/** A tenant as the manager reports it. */
export interface Tenant {
  name: string;
  enabled: boolean;
  vip: string;
  interface: string;
  workers: Worker[];
  status: 'running' | 'stopped' | 'degraded' | 'config-missing';
}

/** One worker of a tenant. */
export interface Worker {
  node_id: string;
  host: string;
  ports: {
    http: number;
    https: number;
    metrics: number;
    sync: number;
    dns: number;
  };
}

/** A routing endpoint with its effective weight. */
export interface Endpoint {
  name: string;
  address: string;
  effective_weight: number;
  healthy: boolean;
}

/** A route as the tenant API reports it. */
export interface Route {
  name: string;
  match: { path: string; type: 'prefix' | 'exact' };
  scope: { region: string };
  affinity: { method: string; cookie_name?: string; header_name?: string; ttl: string };
  endpoints: Endpoint[];
  rollout: RolloutState | null;
}

/** The current rolling-update state. */
export interface RolloutState {
  state:
    | 'idle'
    | 'in_progress'
    | 'paused'
    | 'rolling_back'
    | 'rolled_back'
    | 'completed'
    | 'failed';
  current_step: number;
  steps: number[];
  green_percent: number;
  started_at?: string;
  step_started_at?: string;
  metrics?: {
    window: { error_rate: number; latency_p99_ms: number; samples: number };
  };
  rollback: { reason: string; at: string } | null;
}

/** A DNS record in a zone. */
export interface DnsRecord {
  name: string;
  type: 'A' | 'AAAA' | 'CNAME' | 'SRV';
  value?: string;
  ttl?: number;
  priority?: number;
  weight?: number;
  port?: number;
  target?: string;
}

/** A DNS zone and its records. */
export interface DnsZone {
  zone: string;
  records: DnsRecord[];
}

/** The aggregate health of a tenant's workers. */
export interface Health {
  tenant: string;
  status: 'healthy' | 'degraded' | 'unhealthy';
  workers: {
    node_id: string;
    role: 'active' | 'standby' | 'unknown';
    owns_vip: boolean;
    peer_up: boolean;
  }[];
  endpoints: Endpoint[];
}

/** Lists every tenant. */
export function listTenants() {
  return request<Tenant[]>('/tenants');
}

/** Gets one tenant. */
export function getTenant(name: string) {
  return request<Tenant>(`/tenants/${encodeURIComponent(name)}`);
}

/** Gets a tenant's health. */
export function getTenantHealth(name: string) {
  return request<Health>(`/tenants/${encodeURIComponent(name)}/health`);
}

/** Triggers a configuration reload. */
export function reloadTenant(name: string) {
  return request<void>(`/tenants/${encodeURIComponent(name)}/reload`, { method: 'POST' });
}

/** Lists a tenant's routes. */
export function listRoutes(tenant: string) {
  return request<Route[]>(`/tenants/${encodeURIComponent(tenant)}/routes`);
}

/** Gets the current rollout for a tenant. */
export function getRollout(tenant: string) {
  return request<RolloutState>(`/tenants/${encodeURIComponent(tenant)}/rollout`);
}

/** Advances a rollout by one step. */
export function advanceRollout(tenant: string) {
  return request<RolloutState>(`/tenants/${encodeURIComponent(tenant)}/rollout/advance`, {
    method: 'POST',
  });
}

/** Triggers an immediate rollback. */
export function rollbackRollout(tenant: string) {
  return request<RolloutState>(`/tenants/${encodeURIComponent(tenant)}/rollout/rollback`, {
    method: 'POST',
  });
}

/** Lists the DNS zones. */
export function listZones(tenant: string) {
  return request<DnsZone[]>(`/tenants/${encodeURIComponent(tenant)}/dns/zones`);
}

/** Adds a DNS record. */
export function addRecord(tenant: string, zone: string, record: DnsRecord) {
  return request<DnsZone>(
    `/tenants/${encodeURIComponent(tenant)}/dns/zones/${encodeURIComponent(zone)}/records`,
    { method: 'POST', body: JSON.stringify(record) },
  );
}

/** Deletes a DNS record. */
export function deleteRecord(tenant: string, zone: string, name: string) {
  return request<void>(
    `/tenants/${encodeURIComponent(tenant)}/dns/zones/${encodeURIComponent(zone)}/records/${encodeURIComponent(name)}`,
    { method: 'DELETE' },
  );
}
