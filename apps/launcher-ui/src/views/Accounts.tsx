// Accounts — who launches Minecraft. Public metadata only: this view can
// never touch tokens because no command returns them (Phase 9 §27/§33).
// The Microsoft flow shows Microsoft's REAL device code + verification URL;
// we never render a fake login form (spec §24/§52).
import { useCallback, useEffect, useRef, useState } from "react";
import {
  accountAddOffline,
  accountList,
  accountLogout,
  accountMicrosoftPoll,
  accountMicrosoftStart,
  accountRefresh,
  accountRemove,
  accountSelect,
  toErrorMessage,
  type AccountDto,
} from "../api";
import { Banner, Button, ConfirmDialog, Dialog, EmptyState, Field, Spinner } from "../components/ui";

const STATUS_TEXT: Record<string, string> = {
  "signed-out": "Signed out",
  authenticated: "Authenticated",
  refreshing: "Refreshing…",
  expired: "Expired",
  "reauth-required": "Sign in again required",
  error: "Error",
};

/** Deterministic fallback avatar: hue derived from the stable UUID, initial
 *  from the display name. Same account ⇒ same avatar forever (spec §56). */
function Avatar({ account }: { account: AccountDto }) {
  const hue = [...account.uuid].reduce((acc, ch) => (acc * 31 + ch.charCodeAt(0)) % 360, 7);
  const initial = (account.display_name || account.username || "?").trim().charAt(0).toUpperCase();
  return (
    <span
      className="account-avatar"
      aria-hidden="true"
      style={{
        background: `linear-gradient(135deg, hsl(${hue} 60% 30%), hsl(${(hue + 40) % 360} 55% 18%))`,
      }}
    >
      {initial}
    </span>
  );
}

export function Accounts() {
  const [accounts, setAccounts] = useState<AccountDto[] | null>(null);
  const [notice, setNotice] = useState<{ kind: "info" | "warn" | "error"; text: string } | null>(
    null,
  );
  const [busyId, setBusyId] = useState<string | null>(null);

  // Offline-profile dialog.
  const [showOffline, setShowOffline] = useState(false);
  const [offlineName, setOfflineName] = useState("");
  const [offlineAdding, setOfflineAdding] = useState(false);

  // Microsoft device-code dialog + bounded poll loop.
  const [msa, setMsa] = useState<{
    deviceCode: string;
    userCode: string;
    verificationUri: string;
    intervalSecs: number;
    state: "waiting" | "slow-down" | "expired" | "denied" | "error";
  } | null>(null);
  const pollTimer = useRef<number | null>(null);
  const pollAttempts = useRef(0);

  // Removal requires an explicit armed confirmation.
  const [confirmRemove, setConfirmRemove] = useState<AccountDto | null>(null);
  const [removeArmed, setRemoveArmed] = useState(false);
  const [removing, setRemoving] = useState(false);

  const refresh = useCallback(() => {
    accountList()
      .then(setAccounts)
      .catch((e) => {
        setAccounts([]);
        setNotice({ kind: "error", text: toErrorMessage(e) });
      });
  }, []);

  useEffect(() => {
    refresh();
    return () => {
      if (pollTimer.current !== null) window.clearInterval(pollTimer.current);
    };
  }, [refresh]);

  // ---- actions ----------------------------------------------------------------

  async function select(account: AccountDto) {
    setBusyId(account.id);
    try {
      await accountSelect(account.id);
      refresh();
    } catch (e) {
      setNotice({ kind: "error", text: toErrorMessage(e) });
    } finally {
      setBusyId(null);
    }
  }

  async function logout(account: AccountDto) {
    setBusyId(account.id);
    try {
      await accountLogout(account.id);
      setNotice({ kind: "info", text: `${account.username} signed out.` });
      refresh();
    } catch (e) {
      setNotice({ kind: "error", text: toErrorMessage(e) });
    } finally {
      setBusyId(null);
    }
  }

  async function doRefresh(account: AccountDto) {
    setBusyId(account.id);
    try {
      await accountRefresh(account.id);
      setNotice({ kind: "info", text: `${account.username}: session refreshed.` });
      refresh();
    } catch (e) {
      // Refresh failure is expected sometimes; the backend flips status to
      // reauth-required — surface the reason without crashing anything.
      setNotice({ kind: "warn", text: toErrorMessage(e) });
      refresh();
    } finally {
      setBusyId(null);
    }
  }

  async function addOffline() {
    setOfflineAdding(true);
    try {
      const created = await accountAddOffline(offlineName.trim(), offlineName.trim());
      setShowOffline(false);
      setOfflineName("");
      if (!accounts?.some((a) => a.active)) await accountSelect(created.id).catch(() => undefined);
      setNotice({ kind: "info", text: `Offline profile ${created.username} created.` });
      refresh();
    } catch (e) {
      setNotice({ kind: "error", text: toErrorMessage(e) });
    } finally {
      setOfflineAdding(false);
    }
  }

  async function removeConfirmed() {
    if (!confirmRemove) return;
    setRemoving(true);
    try {
      await accountRemove(confirmRemove.id);
      setNotice({ kind: "info", text: `${confirmRemove.username} removed.` });
      setConfirmRemove(null);
      setRemoveArmed(false);
      refresh();
    } catch (e) {
      setNotice({ kind: "error", text: toErrorMessage(e) });
    } finally {
      setRemoving(false);
    }
  }

  // ---- Microsoft device flow ---------------------------------------------------

  async function startMicrosoft() {
    setNotice(null);
    try {
      const start = await accountMicrosoftStart();
      setMsa({
        deviceCode: start.device_code,
        userCode: start.user_code,
        verificationUri: start.verification_uri,
        intervalSecs: start.interval_secs,
        state: "waiting",
      });
      pollAttempts.current = 0;
      schedulePoll(start.device_code, start.interval_secs);
    } catch (e) {
      setNotice({ kind: "error", text: toErrorMessage(e) });
    }
  }

  function schedulePoll(deviceCode: string, intervalSecs: number) {
    if (pollTimer.current !== null) window.clearInterval(pollTimer.current);
    pollTimer.current = window.setInterval(() => void pollOnce(deviceCode), intervalSecs * 1000);
  }

  /** One poll tick. Hard-bounded attempts so the UI can never hang forever
   *  (spec §61): after ~5 minutes of waiting we stop and say why. */
  async function pollOnce(deviceCode: string) {
    pollAttempts.current += 1;
    if (pollAttempts.current > 100) {
      stopPoll("expired");
      return;
    }
    try {
      const [state, account] = await accountMicrosoftPoll(deviceCode);
      if (state === "ok" && account) {
        stopPoll("waiting");
        setMsa(null);
        setNotice({ kind: "info", text: `Signed in as ${account.username}.` });
        refresh();
      } else if (state === "authorization_pending") {
        /* keep waiting */
      } else if (state === "slow_down") {
        setMsa((prev) => (prev ? { ...prev, state: "slow-down" } : prev));
      } else {
        // access_denied / expired_token / anything terminal
        const next = state.includes("expired") ? "expired" : state === "access_denied" ? "denied" : "error";
        stopPoll(next as "expired" | "denied" | "error");
      }
    } catch (e) {
      // Network hiccup during polling is recoverable — show it but keep trying.
      setNotice({ kind: "warn", text: toErrorMessage(e) });
    }
  }

  function stopPoll(state: "waiting" | "slow-down" | "expired" | "denied" | "error") {
    if (pollTimer.current !== null) {
      window.clearInterval(pollTimer.current);
      pollTimer.current = null;
    }
    if (state !== "waiting") {
      setMsa((prev) => (prev ? { ...prev, state } : prev));
    }
  }

  // ---- render ------------------------------------------------------------------

  const loading = accounts === null;
  const list = accounts ?? [];
  const active = list.find((a) => a.active) ?? null;

  const usernameValid = /^[A-Za-z0-9_]{1,16}$/.test(offlineName.trim());

  return (
    <section className="view" aria-label="Accounts">
      <header className="view-head">
        <h1>Accounts</h1>
        <p className="muted">Who launches Minecraft. Instances are configured separately.</p>
      </header>

      {notice && (
        <Banner kind={notice.kind}>
          {notice.text}{" "}
          <button type="button" className="linkish" onClick={() => setNotice(null)}>
            dismiss
          </button>
        </Banner>
      )}

      {loading ? (
        <Spinner label="Loading accounts…" />
      ) : list.length === 0 ? (
        <div className="panel">
          <EmptyState
            title="No account connected"
            hint="Add a Microsoft account for online play, or create an offline profile for singleplayer and LAN."
            action={
              <div className="dialog-actions">
                <Button variant="primary" onClick={() => void startMicrosoft()}>
                  Sign in with Microsoft
                </Button>
                <Button variant="secondary" onClick={() => setShowOffline(true)}>
                  Create offline profile
                </Button>
              </div>
            }
          />
        </div>
      ) : (
        <>
          {active && (
            <div className="account-active panel">
              <Avatar account={active} />
              <div className="account-copy">
                <span className="account-name">{active.display_name || active.username}</span>
                <span className="muted">
                  {STATUS_TEXT[active.status] ?? active.status} · will be used when you press Play
                </span>
              </div>
              <span className={`badge badge-${active.kind}`}>
                {active.kind === "microsoft" ? "Microsoft" : "Offline"}
              </span>
            </div>
          )}

          <div className="account-list">
            {list.map((a) => (
              <div key={a.id} className={`account-row panel${a.active ? " active" : ""}`}>
                <Avatar account={a} />
                <div className="account-copy">
                  <span className="account-name">{a.display_name || a.username}</span>
                  <span className="muted account-meta">
                    {a.username} · {STATUS_TEXT[a.status] ?? a.status}
                    {a.kind === "microsoft" ? "" : " · local only"}
                  </span>
                </div>
                <span className={`badge badge-${a.kind}`}>
                  {a.kind === "microsoft" ? "Microsoft" : "Offline"}
                </span>
                <div className="account-actions">
                  {!a.active && (
                    <Button
                      variant="secondary"
                      disabled={busyId === a.id}
                      onClick={() => void select(a)}
                      ariaLabel={`Switch to ${a.username}`}
                    >
                      Use
                    </Button>
                  )}
                  {a.kind === "microsoft" && (
                    <Button
                      variant="ghost"
                      disabled={busyId === a.id}
                      onClick={() => void doRefresh(a)}
                      ariaLabel={`Refresh session for ${a.username}`}
                    >
                      Refresh
                    </Button>
                  )}
                  {a.kind === "microsoft" && a.status !== "signed-out" && (
                    <Button
                      variant="ghost"
                      disabled={busyId === a.id}
                      onClick={() => void logout(a)}
                      ariaLabel={`Sign out ${a.username}`}
                    >
                      Sign out
                    </Button>
                  )}
                  <Button
                    variant="danger"
                    onClick={() => {
                      setConfirmRemove(a);
                      setRemoveArmed(false);
                    }}
                    ariaLabel={`Remove ${a.username}`}
                  >
                    Remove
                  </Button>
                </div>
              </div>
            ))}
          </div>

          <div className="dialog-actions">
            <Button variant="primary" onClick={() => void startMicrosoft()}>
              Add Microsoft account
            </Button>
            <Button variant="secondary" onClick={() => setShowOffline(true)}>
              Create offline profile
            </Button>
          </div>
        </>
      )}

      {/* ---- offline profile dialog ---- */}
      {showOffline && (
        <Dialog title="Create offline profile" onClose={() => setShowOffline(false)}>
          <form
            className="dialog-body"
            onSubmit={(e) => {
              e.preventDefault();
              if (usernameValid) void addOffline();
            }}
          >
            <Field label="Username" error={offlineName && !usernameValid ? "1–16 characters of A–Z, a–z, 0–9, _" : undefined}>
              <input
                value={offlineName}
                autoFocus
                onChange={(e) => setOfflineName(e.target.value)}
                placeholder="Steve"
                maxLength={16}
                pattern="[A-Za-z0-9_]{1,16}"
                aria-label="Offline profile username"
              />
            </Field>
            <p className="muted">
              This creates an explicit <strong>Offline / Local</strong> profile: singleplayer and
              LAN only. Authenticated servers reject it — Isekaiyo never fakes a premium login.
            </p>
            <div className="dialog-actions">
              <Button variant="ghost" onClick={() => setShowOffline(false)}>
                Cancel
              </Button>
              <Button variant="primary" type="submit" disabled={!usernameValid || offlineAdding}>
                {offlineAdding ? "Creating…" : "Create"}
              </Button>
            </div>
          </form>
        </Dialog>
      )}

      {/* ---- Microsoft device-code dialog ---- */}
      {msa && (
        <Dialog title="Sign in with Microsoft" onClose={() => { stopPoll("waiting"); setMsa(null); }}>
          <div className="dialog-body">
            {msa.state === "waiting" || msa.state === "slow-down" ? (
              <>
                <p>
                  In your browser, open{" "}
                  <strong>{msa.verificationUri}</strong> and enter this code:
                </p>
                <p className="device-code" aria-label="Device code">
                  {msa.user_code}
                </p>
                <p className="muted">
                  Isekaiyo never sees your Microsoft password — authentication happens on
                  microsoft.com. This dialog waits while Minecraft entitlements are verified.
                </p>
                {msa.state === "slow-down" && (
                  <Banner kind="warn">Microsoft asked us to slow down polling; still waiting…</Banner>
                )}
                <Spinner label="Waiting for you to finish in the browser…" />
                <div className="dialog-actions">
                  <Button variant="ghost" onClick={() => setMsa(null)}>
                    Cancel
                  </Button>
                </div>
              </>
            ) : (
              <>
                <Banner kind="error">
                  {msa.state === "denied"
                    ? "Authentication was denied — you declined the sign-in request."
                    : msa.state === "expired"
                      ? "The code expired before sign-in completed. Start again to get a fresh code."
                      : "Authentication failed. Check your connection and try again."}
                </Banner>
                <div className="dialog-actions">
                  <Button variant="ghost" onClick={() => setMsa(null)}>
                    Close
                  </Button>
                  <Button variant="primary" onClick={() => void startMicrosoft()}>
                    Try again
                  </Button>
                </div>
              </>
            )}
          </div>
        </Dialog>
      )}

      {/* ---- removal confirmation (armed two-step) ---- */}
      {confirmRemove && (
        <ConfirmDialog
          title={`Remove ${confirmRemove.username}?`}
          body={
            removeArmed
              ? "Final step: this deletes stored credentials and account metadata. Your instances are NOT touched."
              : "Stored credentials and account metadata will be deleted. Your instances and worlds are untouched."
          }
          confirmLabel={removeArmed ? "Delete permanently" : "Continue"}
          busy={removing}
          onConfirm={() => (removeArmed ? void removeConfirmed() : setRemoveArmed(true))}
          onCancel={() => {
            setConfirmRemove(null);
            setRemoveArmed(false);
          }}
        />
      )}
    </section>
  );
}
