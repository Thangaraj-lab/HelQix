/** Branch lineage tree from Layer-11 branch records. */
import type { BranchNode } from "../../api/types";
import { Badge } from "../common";
import { formatLogicalTime } from "../../utils/format";
import "./branch.css";

interface Props { branches: ReadonlyArray<BranchNode>; selectedId: string | null; onSelect(id: string): void }

export function BranchTree({ branches, selectedId, onSelect }: Props) {
  const childrenOf = (parent: string | null) => branches.filter((b) => b.parentBranchId === parent);
  const roots = childrenOf(null);

  function render(branch: BranchNode, depth: number, isLast: boolean, prefix: string): JSX.Element {
    const kids = childrenOf(branch.id);
    return (
      <div key={branch.id}>
        <button className="branch-row" role="treeitem" aria-selected={selectedId === branch.id} style={{ paddingLeft: 10 + depth * 18 }} onClick={() => onSelect(branch.id)}>
          <span className="branch-row__glyph" aria-hidden>{depth === 0 ? "◉ ORIGINAL" : `${prefix}${isLast ? "└─" : "├─"} BRANCH ${branch.id.slice(0, 6).toUpperCase()}`}</span>
          <span style={{ marginLeft: "auto", display: "flex", gap: 8, alignItems: "center" }}>
            <span className="mono" style={{ fontSize: "var(--hq-text-xs)", color: "var(--hq-text-muted)" }}>@{formatLogicalTime(branch.originTime?.nanos)}</span>
            <Badge tone={branch.status === "completed" ? "success" : branch.status === "faulted" ? "failure" : branch.status === "running" ? "active" : "normal"}>{branch.status}</Badge>
          </span>
        </button>
        <div role="group">
          {kids.map((k, i) => render(k, depth + 1, i === kids.length - 1, depth === 0 ? "" : `${isLast ? "    " : "│   "}`))}
        </div>
      </div>
    );
  }

  return (
    <div role="tree" aria-label="Branch lineage">
      {roots.length === 0 && branches.length === 0 ? <p className="page__subtitle" style={{ padding: 10 }}>No branches recorded for this project yet.</p> : null}
      {branches.filter((b) => b.parentBranchId === null).map((b, i, arr) => render(b, 0, i === arr.length - 1, ""))}
      {branches.filter((b) => b.parentBranchId !== null && !branches.some((p) => p.id === b.parentBranchId)).map((b) => render(b, 0, true, ""))}
    </div>
  );
}
