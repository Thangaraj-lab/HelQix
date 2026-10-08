/** Settings — secondary to engineering work. Preferences persist locally; anything server-owned is marked. */
import { useEffect, useState } from "react";
import { PageHeader } from "../../components/common";

interface UiPrefs { units: "si" | "imperial"; reduceMotion: boolean; timelineDensity: "compact" | "comfortable" }
const PREFS_KEY = "helqix.ui-prefs";

function loadPrefs(): UiPrefs {
  try {
    const raw = localStorage.getItem(PREFS_KEY);
    if (raw) return { units: "si", reduceMotion: false, timelineDensity: "compact", ...JSON.parse(raw) as Partial<UiPrefs> };
  } catch { /* ignore */ }
  return { units: "si", reduceMotion: false, timelineDensity: "compact" };
}

export function SettingsPage() {
  const [prefs, setPrefs] = useState<UiPrefs>(loadPrefs);
  useEffect(() => {
    localStorage.setItem(PREFS_KEY, JSON.stringify(prefs));
    document.documentElement.dataset.units = prefs.units;
    document.documentElement.style.setProperty("--hq-timeline-density", prefs.timelineDensity === "compact" ? "4px" : "10px");
  }, [prefs]);

  return (
    <div className="page" style={{ maxWidth: 760 }}>
      <PageHeader title="Settings" subtitle="Frontend preferences are local. Account, workspace membership, and connection settings are owned by the platform backend." />
      <section className="hq-panel" style={{ marginBottom: "var(--hq-sp-4)" }}>
        <div className="hq-panel__header"><span className="label">Units & visualization</span></div>
        <div className="hq-panel__body" style={{ display: "grid", gap: "var(--hq-sp-4)" }}>
          <label style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span>Measurement units</span>
            <select className="hq-select" style={{ width: 160 }} value={prefs.units} onChange={(e) => setPrefs({ ...prefs, units: e.target.value as UiPrefs["units"] })}>
              <option value="si">SI</option><option value="imperial">Imperial</option>
            </select>
          </label>
          <label style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span>Timeline density</span>
            <select className="hq-select" style={{ width: 160 }} value={prefs.timelineDensity} onChange={(e) => setPrefs({ ...prefs, timelineDensity: e.target.value as UiPrefs["timelineDensity"] })}>
              <option value="compact">Compact</option><option value="comfortable">Comfortable</option>
            </select>
          </label>
          <label style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span>Reduce motion</span>
            <input type="checkbox" checked={prefs.reduceMotion} onChange={(e) => setPrefs({ ...prefs, reduceMotion: e.target.checked })} />
          </label>
        </div>
      </section>
      <section className="hq-panel">
        <div className="hq-panel__header"><span className="label">Account · Security · Connection</span></div>
        <div className="hq-panel__body page__subtitle">
          Managed by the HELQIX platform services. These controls become available when the Layer&nbsp;22
          account-settings endpoints are connected — they are intentionally not simulated here.
        </div>
      </section>
    </div>
  );
}
