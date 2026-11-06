// LISTEN / NOTIFY notifications per session, kept in memory. Fed by the
// backend's `notificationReceived` event: from the Notifications view's
// listener connection and from LISTEN typed in the console.
import { create } from "zustand";

import { api, events, type NotificationPayload, type NotificationReceived } from "@/lib/ipc";
import { useSettingsStore } from "@/stores/settings";

export type NotificationEntry = NotificationPayload & {
  id: number;
  source: NotificationReceived["source"];
};

export type SessionNotifications = {
  entries: NotificationEntry[];
  /** Channels the view's listener is subscribed to. */
  channels: string[];
  /** Arrived while the view wasn't showing. */
  unread: number;
  /** Lost because the UI fell behind, or dropped past the buffer. */
  dropped: number;
  paused: boolean;
};

const EMPTY: SessionNotifications = { entries: [], channels: [], unread: 0, dropped: 0, paused: false };

let nextId = 1;

interface NotificationsState {
  bySession: Record<string, SessionNotifications>;
  /** Sessions whose Notifications view is on screen (no unread count). */
  watching: Record<string, boolean>;
  listen: (sessionId: string, channel: string) => Promise<void>;
  unlisten: (sessionId: string, channel: string) => Promise<void>;
  /** Reads the listener's channels again (after a reconnect: none). */
  refreshChannels: (sessionId: string) => Promise<void>;
  send: (sessionId: string, channel: string, payload: string) => Promise<void>;
  setPaused: (sessionId: string, paused: boolean) => void;
  clear: (sessionId: string) => void;
  setWatching: (sessionId: string, watching: boolean) => void;
  receive: (event: NotificationReceived) => void;
}

const patch = (
  s: NotificationsState,
  sessionId: string,
  change: (n: SessionNotifications) => Partial<SessionNotifications>,
) => {
  const current = s.bySession[sessionId] ?? EMPTY;
  return { bySession: { ...s.bySession, [sessionId]: { ...current, ...change(current) } } };
};

export const useNotificationsStore = create<NotificationsState>()((set, get) => ({
  bySession: {},
  watching: {},

  listen: async (sessionId, channel) => {
    const channels = await api.listenChannel(sessionId, channel);
    set((s) => patch(s, sessionId, () => ({ channels })));
  },
  unlisten: async (sessionId, channel) => {
    const channels = await api.unlistenChannel(sessionId, channel);
    set((s) => patch(s, sessionId, () => ({ channels })));
  },
  refreshChannels: async (sessionId) => {
    const channels = await api.listListenChannels(sessionId);
    set((s) => patch(s, sessionId, () => ({ channels })));
  },
  send: async (sessionId, channel, payload) => {
    await api.sendNotify(sessionId, channel, payload);
  },
  setPaused: (sessionId, paused) => set((s) => patch(s, sessionId, () => ({ paused }))),
  clear: (sessionId) => set((s) => patch(s, sessionId, () => ({ entries: [], unread: 0, dropped: 0 }))),
  setWatching: (sessionId, watching) =>
    set((s) => ({
      watching: { ...s.watching, [sessionId]: watching },
      ...(watching ? patch(s, sessionId, () => ({ unread: 0 })) : {}),
    })),

  receive: (event) => {
    const { sessionId } = event;
    const limit = useSettingsStore.getState().settings?.connection.notificationBuffer ?? 1000;
    const watching = get().watching[sessionId] ?? false;
    set((s) =>
      patch(s, sessionId, (n) => {
        if (!event.notification) return { dropped: n.dropped + event.dropped };
        if (n.paused) return { dropped: n.dropped + 1 };
        const entry: NotificationEntry = { ...event.notification, id: nextId++, source: event.source };
        const all = n.entries.concat(entry);
        const over = Math.max(0, all.length - limit);
        return {
          entries: over ? all.slice(over) : all,
          dropped: n.dropped + over,
          unread: watching ? 0 : n.unread + 1,
        };
      }),
    );
  },
}));

let listening = false;

/** Starts following notifications; call once at startup. */
export function initNotifications() {
  if (listening) return;
  listening = true;
  void events.notificationReceived.listen((e) => useNotificationsStore.getState().receive(e.payload));
}
