import { call, type Account } from "../ipc";

export const accountsList = () => call<Account[]>("accounts_list");

/** Returns the consent URL. Completion arrives as the `auth` event. */
export const accountConnect = () => call<string>("account_connect");

/** Stops the sign-in in flight. A late answer from the browser then adds nothing. */
export const accountConnectCancel = () => call<void>("account_connect_cancel");

export const accountDisconnect = (accountId: string) =>
  call<void>("account_disconnect", { accountId });

export const caldavConnect = (server: string, username: string, password: string) =>
  call<Account>("caldav_connect", { server, username, password });
