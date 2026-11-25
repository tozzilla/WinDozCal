import { createRoot } from "react-dom/client";
import { act } from "react";
import { QueryClientProvider } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";
import { calendarService } from "@/providers/calendarService";
import { queryClient } from "@/providers/queries";
import { CaldavForm } from "./CaldavForm";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(async () => { await new Promise((r) => setTimeout(r, 0)); });

/** Imposta il valore come farebbe la digitazione, così React vede l'evento. */
function type(input: HTMLInputElement, value: string) {
  act(() => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

function mount(onDone = vi.fn()) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<QueryClientProvider client={queryClient}><CaldavForm onDone={onDone} onCancel={vi.fn()} /></QueryClientProvider>));
  const field = (label: string) => host.querySelector<HTMLInputElement>(`input[id$="-${label}"]`)!;
  return { host, root, field, onDone };
}

describe("CaldavForm", () => {
  it("invia URL, utente e password al servizio e azzera la password", async () => {
    const spy = vi.spyOn(calendarService, "connectCaldav");
    const { host, root, field, onDone } = mount();
    const submit = host.querySelector<HTMLButtonElement>("button[type=submit]")!;
    expect(submit.disabled).toBe(true);
    type(field("url"), "https://dav.example.com/dav");
    type(field("user"), "anna");
    type(field("pwd"), "segreta");
    expect(field("pwd").type).toBe("password");
    expect(submit.disabled).toBe(false);
    act(() => host.querySelector("form")!.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true })));
    await flush();
    expect(spy).toHaveBeenCalledWith("https://dav.example.com/dav", "anna", "segreta");
    expect(field("pwd").value).toBe("");
    expect(onDone).toHaveBeenCalled();
    act(() => root.unmount());
    spy.mockRestore();
  });

  it("avvisa che con http:// la password viaggia in chiaro", () => {
    const { host, root, field } = mount();
    expect(host.textContent).not.toContain("in chiaro");
    type(field("url"), "http://192.168.1.10/dav");
    expect(host.textContent).toContain("in chiaro");
    act(() => root.unmount());
  });
});
