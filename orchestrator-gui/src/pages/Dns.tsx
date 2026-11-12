import { useQuery } from '@tanstack/react-query';
import { listTenants, listZones } from '../api/endpoints';
import DnsZoneEditor from '../components/DnsZoneEditor';

/** The DNS zone editor. */
export default function Dns() {
  const tenants = useQuery({ queryKey: ['tenants'], queryFn: listTenants });
  const first = tenants.data?.[0]?.name ?? '';

  const zones = useQuery({
    queryKey: ['zones', first],
    queryFn: () => listZones(first),
    enabled: first !== '',
  });

  if (!first) return <p>No tenants are registered.</p>;

  return (
    <main>
      <h1>DNS zones</h1>
      <DnsZoneEditor tenant={first} zones={zones.data ?? []} />
    </main>
  );
}
