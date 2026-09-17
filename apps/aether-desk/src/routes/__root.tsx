import { Outlet, createRootRoute } from "@tanstack/react-router";
import { AppShell } from "@/components/app-shell";

export const Route = createRootRoute({
  component: Root,
});

function Root() {
  return (
    <AppShell>
      <Outlet />
    </AppShell>
  );
}
