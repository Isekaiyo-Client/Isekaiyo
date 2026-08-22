// About — real application identity: version, platform, license, links.
// Everything shown here comes from the core or from repository facts; nothing
// is aspirational.
import type { SystemInfo } from "../api";

export function About({ info }: { info: SystemInfo | null }) {
  return (
    <section className="view" aria-label="About">
      <header className="view-head">
        <h1>About</h1>
        <p className="muted">Isekaiyo — an independent, open-source Minecraft launcher and client platform.</p>
      </header>

      <div className="panel">
        <h2>This build</h2>
        {info ? (
          <dl className="about-list">
            <dt>Version</dt>
            <dd>{info.app_version}</dd>
            <dt>Platform</dt>
            <dd>{info.target}</dd>
            <dt>Profile</dt>
            <dd>{info.profile}</dd>
          </dl>
        ) : (
          <p className="muted">Connecting to core…</p>
        )}
      </div>

      <div className="panel">
        <h2>Project</h2>
        <p className="muted">
          Isekaiyo is developed in the open by its community. It is not affiliated with Mojang
          Studios or Microsoft, and does not distribute Minecraft assets.
        </p>
        <dl className="about-list">
          <dt>License</dt>
          <dd>GPL-3.0-or-later</dd>
          <dt>Source</dt>
          <dd>
            <a
              className="linkish-ext"
              href="https://github.com/Isekaiyo-Client/Isekaiyo"
              target="_blank"
              rel="noreferrer"
            >
              github.com/Isekaiyo-Client/Isekaiyo
            </a>
          </dd>
        </dl>
      </div>
    </section>
  );
}
