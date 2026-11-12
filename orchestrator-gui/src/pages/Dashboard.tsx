import { useQuery } from '@tanstack/react-query';
import { listTenants, type Tenant } from '../api/endpoints';
import TenantSwitcher from '../components/TenantSwitcher';

/** The health dashboard — the landing page. */
export default function Dashboard() {
  const tenants = useQuery({ queryKey: ['tenants'], queryFn: listTenants });

  if (tenants.isLoading) return <p>Loading…</p>;
  if (tenants.isError) return <p role="alert">Could not load the tenants.</p>;

  return (
    <main>
      <h1>Tenants</h1>
      <TenantSwitcher tenants={tenants.data ?? []} />
      <table>
        <thead>
          <tr>
            <th>Tenant</th>
            <th>Status</th>
            <th>VIP</th>
            <th>Workers</th>
          </tr>
        </thead>
        <tbody>
          {(tenants.data ?? []).map((tenant: Tenant) => (
            <tr key={tenant.name}>
              <td>{tenant.name}</td>
              <td>{tenant.status}</td>
              <td>{tenant.vip}</td>
              <td>{tenant.workers.length}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </main>
  );
}
