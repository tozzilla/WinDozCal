import type { Dictionary } from "..";

export const errors: Dictionary["errors"] = {
  database: "Local database error",
  io: "Disk read or write error",
  tauri: "Application error",
  not_found: "Item not found",
  invalid_input: "Invalid data",
  not_implemented: "Not available yet",
  credentials: "Could not access Windows credentials",
  configuration: "Missing configuration in this build",
  provider: "The server rejected the request",
  auth_required: "The account needs to sign in again",
  network: "Network error",
  rate_limited: "Too many requests to the server, try again shortly",
  sync_cursor_expired: "Synchronization must restart from scratch",
  internal: "Internal error",
};
