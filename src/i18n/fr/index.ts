import type { PartialDictionary } from "..";
import { shell } from "./shell";
import { calendar } from "./calendar";
import { events } from "./events";
import { settings } from "./settings";
import { accounts } from "./accounts";
import { errors } from "./errors";

export const fr: PartialDictionary = { shell, calendar, events, settings, accounts, errors };
