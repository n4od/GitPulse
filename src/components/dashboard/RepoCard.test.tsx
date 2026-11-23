import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import { RepoCard } from "@/components/dashboard/RepoCard";
import { makeCard, makeRepo, makeRun } from "@/test/fixtures";
import { useToastStore } from "@/stores/toasts";

describe("RepoCard", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue(undefined);
    useToastStore.setState({ toasts: [] });
  });

  it("shows the repository and its mapped status", () => {
    render(
      <RepoCard
        card={makeCard({
          repo: makeRepo({ fullName: "acme/api" }),
          recentRuns: [makeRun({ status: "completed", conclusion: "failure" })],
        })}
        onRefresh={vi.fn()}
        onToggleMonitored={vi.fn()}
        onOpen={vi.fn()}
      />,
    );

    expect(screen.getByText("acme/api")).toBeInTheDocument();
    expect(screen.getByText("Failed")).toBeInTheDocument();
  });

  it("falls back to a no-runs badge for a fresh repository", () => {
    render(
      <RepoCard
        card={makeCard({ recentRuns: [] })}
        onRefresh={vi.fn()}
        onToggleMonitored={vi.fn()}
        onOpen={vi.fn()}
      />,
    );

    expect(screen.getByText("No runs")).toBeInTheDocument();
  });

  it("marks a private repository", () => {
    render(
      <RepoCard
        card={makeCard({ repo: makeRepo({ isPrivate: true }) })}
        onRefresh={vi.fn()}
        onToggleMonitored={vi.fn()}
        onOpen={vi.fn()}
      />,
    );

    expect(screen.getByText("private")).toBeInTheDocument();
  });

  it("marks a muted repository", () => {
    render(
      <RepoCard
        card={makeCard({ repo: makeRepo({ monitored: false }) })}
        onRefresh={vi.fn()}
        onToggleMonitored={vi.fn()}
        onOpen={vi.fn()}
      />,
    );

    expect(screen.getByText("muted")).toBeInTheDocument();
  });

  it("asks for a refresh", async () => {
    const onRefresh = vi.fn();

    render(
      <RepoCard
        card={makeCard()}
        onRefresh={onRefresh}
        onToggleMonitored={vi.fn()}
        onOpen={vi.fn()}
      />,
    );
    await userEvent.click(screen.getByRole("button", { name: /refresh acme\/api/i }));

    expect(onRefresh).toHaveBeenCalledWith(1);
  });

  it("unmutes a muted repository", async () => {
    const onToggleMonitored = vi.fn();

    render(
      <RepoCard
        card={makeCard({ repo: makeRepo({ id: 4, monitored: false }) })}
        onRefresh={vi.fn()}
        onToggleMonitored={onToggleMonitored}
        onOpen={vi.fn()}
      />,
    );

    await userEvent.click(screen.getByRole("button", { name: /mute acme\/api/i }));

    expect(onToggleMonitored).toHaveBeenCalledWith(4, true);
  });

  it("opens the run in the browser through the validated command", async () => {
    render(
      <RepoCard
        card={makeCard()}
        onRefresh={vi.fn()}
        onToggleMonitored={vi.fn()}
        onOpen={vi.fn()}
      />,
    );
    await userEvent.click(
      screen.getByRole("button", { name: /open acme\/api on github/i }),
    );

    expect(invoke).toHaveBeenCalledWith("system_open_url", {
      url: "https://github.com/acme/api/actions/runs/500",
    });
  });

  it("reports how many runs are failing", () => {
    render(
      <RepoCard
        card={makeCard({ failingRuns: 3 })}
        onRefresh={vi.fn()}
        onToggleMonitored={vi.fn()}
        onOpen={vi.fn()}
      />,
    );

    expect(screen.getByText(/3 failing runs/)).toBeInTheDocument();
  });

  it("badges the failing run, not whichever run is newest", () => {
    // CI failed, then Release succeeded on a newer tag. Badging the newest run
    // read "Success" beside "1 failing run", and disagreed with the tray. The
    // badge and the line under it now both come from the failing workflow.
    render(
      <RepoCard
        card={makeCard({
          failingRuns: 1,
          recentRuns: [
            makeRun({ id: 2, workflowName: "Release", conclusion: "success" }),
            makeRun({ id: 1, workflowName: "CI", conclusion: "failure" }),
          ],
          failingRun: makeRun({
            id: 1,
            workflowName: "CI",
            conclusion: "failure",
          }),
        })}
        onRefresh={vi.fn()}
        onToggleMonitored={vi.fn()}
        onOpen={vi.fn()}
      />,
    );

    expect(screen.getByText("Failed")).toBeInTheDocument();
    expect(screen.getByText("CI")).toBeInTheDocument();
    expect(screen.queryByText("Success")).not.toBeInTheDocument();
    expect(screen.queryByText("Release")).not.toBeInTheDocument();
  });

  it("renders the same card at the compact density", () => {
    const { container } = render(
      <RepoCard
        card={makeCard({
          repo: makeRepo({ fullName: "acme/api" }),
          recentRuns: [makeRun(), makeRun({ id: 2 })],
        })}
        density="compact"
        onRefresh={vi.fn()}
        onToggleMonitored={vi.fn()}
        onOpen={vi.fn()}
      />,
    );

    expect(screen.getByText("acme/api")).toBeInTheDocument();
    // The compact card tightens the padding rather than dropping content.
    expect(container.querySelector(".pt-2\\.5")).not.toBeNull();
    expect(container.querySelector("svg.h-6")).not.toBeNull();
  });
});
