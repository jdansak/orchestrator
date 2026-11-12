import { useParams } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { getTenant, listRoutes } from '../api/endpoints';
import TreeView from '../components/TreeView';
import SankeyDiagram from '../components/SankeyDiagram';

/** One tenant: its hierarchy and its traffic split. */
export default function TenantDetail() {
  const { name = '' } = useParams();

  const tenant = useQuery({ queryKey: ['tenant', name], queryFn: () => getTenant(name) });
  const routes = useQuery({ queryKey: ['routes', name], queryFn: () => listRoutes(name) });

  if (tenant.isLoading) return <p>Loading…</p>;
  if (tenant.isError) return <p role="alert">Could not load this tenant.</p>;

  return (
    <main>
      <h1>{tenant.data?.name}</h1>
      <TreeView routes={routes.data ?? []} />
      <SankeyDiagram routes={routes.data ?? []} />
    </main>
  );
}
