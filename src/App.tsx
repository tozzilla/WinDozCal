import { QueryClientProvider } from "@tanstack/react-query";
import { WelcomeScreen } from "@/accounts/WelcomeScreen";
import { CalendarSurface } from "@/components/CalendarSurface";
import { Header } from "@/components/Header";
import { Sidebar } from "@/components/Sidebar";
import { EventEditor } from "@/events/EventEditor";
import { useKeyboardShortcuts } from "@/hooks/useKeyboardShortcuts";
import { useTheme } from "@/hooks/useTheme";
import { queryClient, useAccounts } from "@/providers/queries";
import { SettingsPage } from "@/settings/SettingsPage";
import { useUiStore } from "@/stores/uiStore";

function Shell() {
  useTheme();
  useKeyboardShortcuts();
  const settingsOpen = useUiStore((s) => s.settingsOpen);
  const { data: accounts, isPending } = useAccounts();

  if (isPending) return null;
  if (accounts?.length === 0) return <WelcomeScreen />;
  if (settingsOpen) return <SettingsPage />;

  return (
    <div className="flex h-full">
      <Sidebar />
      <div className="flex min-w-0 flex-1 flex-col">
        <Header />
        <CalendarSurface />
      </div>
      <EventEditor />
    </div>
  );
}

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <Shell />
    </QueryClientProvider>
  );
}
