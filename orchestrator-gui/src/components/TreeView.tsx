import { useMemo, type ReactNode } from 'react';
import type { Route } from '../api/endpoints';

interface TreeNode {
  name: string;
  path: string;
  weight: number;
  children: TreeNode[];
}

/** Builds the hierarchy tree from the route table. */
function buildTree(routes: Route[]): TreeNode {
  const root: TreeNode = { name: 'root', path: '', weight: 1, children: [] };

  for (const route of routes) {
    for (const endpoint of route.endpoints) {
      const parts = endpoint.name.split('/');
      let node = root;
      let path = '';

      for (const part of parts) {
        path = path ? `${path}/${part}` : part;
        let child = node.children.find((candidate) => candidate.name === part);
        if (!child) {
          child = { name: part, path, weight: endpoint.effective_weight, children: [] };
          node.children.push(child);
        }
        node = child;
      }
    }
  }

  return root;
}

/** The hierarchy tree view (G8.3). */
export default function TreeView({ routes }: { routes: Route[] }) {
  const tree = useMemo(() => buildTree(routes), [routes]);

  function render(node: TreeNode): ReactNode {
    return (
      <li key={node.path}>
        <span>{node.name}</span>
        {node.children.length > 0 ? <ul>{node.children.map(render)}</ul> : null}
      </li>
    );
  }

  return (
    <nav aria-label="Hierarchy">
      <ul>{tree.children.map(render)}</ul>
    </nav>
  );
}
