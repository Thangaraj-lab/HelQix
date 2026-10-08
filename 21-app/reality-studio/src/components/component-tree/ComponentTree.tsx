/** Hierarchical component tree shared by Model / Architecture / Reality Studio. */
import type { ModelComponent } from "../../api/types";

interface Props {
  components: ReadonlyArray<ModelComponent>;
  selectedId: string | null;
  onSelect(id: string): void;
}

export function ComponentTree({ components, selectedId, onSelect }: Props) {
  const childrenOf = (parent: string | null): ModelComponent[] =>
    components.filter((c) => c.parentId === parent);

  function renderNode(node: ModelComponent, depth: number): JSX.Element {
    return (
      <div key={node.id}>
        <button
          className="tree-node"
          role="treeitem"
          aria-selected={selectedId === node.id}
          style={{ paddingLeft: 8 + depth * 16 }}
          onClick={() => onSelect(node.id)}
        >
          <span aria-hidden style={{ marginRight: 6, opacity: 0.6 }}>{depth === 0 ? "▣" : "├"}</span>
          {node.name}
          <span style={{ float: "right", color: "var(--hq-text-muted)", fontSize: "var(--hq-text-xs)" }}>{node.kind}</span>
        </button>
        <div role="group">{childrenOf(node.id).map((c) => renderNode(c, depth + 1))}</div>
      </div>
    );
  }

  const roots = childrenOf(null);
  return (
    <div role="tree" aria-label="System component tree">
      {roots.length === 0 ? <p className="page__subtitle" style={{ padding: 8 }}>No components in model.</p> : roots.map((c) => renderNode(c, 0))}
    </div>
  );
}
