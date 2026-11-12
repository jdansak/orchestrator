import { useQuery } from '@tanstack/react-query';
import { listTenants, getRollout } from '../api/endpoints';
import RollingUpdate from '../components/RollingUpdate';

/** The rolling-update console. */
export default function Rollouts() {
  const tenants = useQuery({ queryKey: ['tenants'], queryFn: listTenants });
  const first = tenants.data?.[0]?.name ?? '';

  const rollout = useQuery({
    queryKey: ['rollout', first],
    queryFn: () => getRollout(first),
    enabled: first !== '',
    refetchInterval: 2_000,
  });

  if (!first) return <p>No tenants are registered.</p>;

  return (
    <main>
      <h1>Rolling updates</h1>
      <RollingUpdate tenant={first} rollout={rollout.data ?? null} />
    </main>
  );
}
