import { useNavigate, useParams } from 'react-router-dom';
import type { Tenant } from '../api/endpoints';

/** Selects which tenant to manage. */
export default function TenantSwitcher({ tenants }: { tenants: Tenant[] }) {
  const navigate = useNavigate();
  const { name } = useParams();

  return (
    <nav aria-label="Tenant">
      <label htmlFor="tenant-switcher">Tenant</label>
      <select
        id="tenant-switcher"
        value={name ?? tenants[0]?.name ?? ''}
        onChange={(event) => navigate(`/tenants/${event.target.value}`)}
      >
        {tenants.map((tenant) => (
          <option key={tenant.name} value={tenant.name}>
            {tenant.name}
          </option>
        ))}
      </select>
    </nav>
  );
}
