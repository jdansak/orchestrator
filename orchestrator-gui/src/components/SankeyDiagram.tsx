import { useMemo } from 'react';
import type { Route } from '../api/endpoints';

interface Flow {
  source: string;
  target: string;
  value: number;
}

/** Builds the Sankey links from the route table. */
function buildFlows(routes: Route[]): Flow[] {
  const flows: Flow[] = [];

  for (const route of routes) {
    for (const endpoint of route.endpoints) {
      const parts = endpoint.name.split('/');
      const target = parts[parts.length - 1] ?? endpoint.name;
      const source = parts.slice(0, -1).join('/') || 'root';
      flows.push({ source, target, value: endpoint.effective_weight * 100 });
    }
  }

  return flows;
}

/** The traffic-split Sankey diagram (G8.4). */
export default function SankeyDiagram({ routes }: { routes: Route[] }) {
  const flows = useMemo(() => buildFlows(routes), [routes]);
  const total = flows.reduce((sum, flow) => sum + flow.value, 0);

  return (
    <figure aria-label="Traffic split">
      <figcaption>Traffic split — {total.toFixed(1)}% routed</figcaption>
      <ul>
        {flows.map((flow) => (
          <li key={`${flow.source}->${flow.target}`}>
            {flow.source} → {flow.target}: {flow.value.toFixed(1)}%
          </li>
        ))}
      </ul>
    </figure>
  );
}
