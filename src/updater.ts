import { useCallback, useEffect, useRef, useState } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type UpdateStatus = "idle" | "checking" | "available" | "downloading" | "ready" | "error";

export type UpdateView = {
  status: UpdateStatus;
  version: string | null;
  percent: number;
  message: string | null;
};

const IDLE: UpdateView = { status: "idle", version: null, percent: 0, message: null };
const CHECK_EVERY = 4 * 60 * 60 * 1000;

export function useUpdater() {
  const [view, setView] = useState<UpdateView>(IDLE);
  const pending = useRef<Update | null>(null);
  const busy = useRef(false);

  const checkNow = useCallback(async (silent = false) => {
    if (busy.current) return;
    busy.current = true;
    setView((prev) => ({ ...prev, status: "checking", message: null }));
    try {
      const next = await check();
      pending.current = next;
      if (next) {
        setView({ status: "available", version: next.version, percent: 0, message: null });
      } else {
        setView({
          status: "idle",
          version: null,
          percent: 0,
          message: silent ? null : "You are on the latest version."
        });
      }
    } catch (reason) {
      pending.current = null;
      setView({ status: "error", version: null, percent: 0, message: String(reason) });
    } finally {
      busy.current = false;
    }
  }, []);

  const install = useCallback(async () => {
    const current = pending.current;
    if (!current || busy.current) return;
    busy.current = true;
    setView((prev) => ({ ...prev, status: "downloading", percent: 0, message: null }));
    let received = 0;
    let total = 0;
    try {
      await current.downloadAndInstall((event) => {
        if (event.event === "Started") {
          total = event.data.contentLength ?? 0;
        } else if (event.event === "Progress") {
          received += event.data.chunkLength;
          if (total > 0) {
            setView((prev) => ({
              ...prev,
              percent: Math.min(99, Math.round((received / total) * 100))
            }));
          }
        }
      });
      setView((prev) => ({ ...prev, status: "ready", percent: 100 }));
      await relaunch();
    } catch (reason) {
      setView((prev) => ({ ...prev, status: "error", message: String(reason) }));
    } finally {
      busy.current = false;
    }
  }, []);

  const dismiss = useCallback(() => {
    pending.current = null;
    setView(IDLE);
  }, []);

  useEffect(() => {
    const first = window.setTimeout(() => void checkNow(true), 2500);
    const timer = window.setInterval(() => void checkNow(true), CHECK_EVERY);
    return () => {
      window.clearTimeout(first);
      window.clearInterval(timer);
    };
  }, [checkNow]);

  return { view, checkNow, install, dismiss };
}
