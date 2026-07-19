import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import GlassSelect from "./GlassSelect.svelte";

const options = [
    { value: "WATCHING", label: "Watching" },
    { value: "COMPLETED", label: "Completed" },
    { value: "PLANNING", label: "Planning" },
] as const;

describe("GlassSelect", () => {
    it("renders a frosted trigger and menu with explicit hoverable options", async () => {
        const user = userEvent.setup();
        const onChange = vi.fn();
        render(GlassSelect, {
            id: "status-filter",
            label: "status",
            value: "WATCHING",
            options,
            onChange,
        });

        const trigger = screen.getByRole("combobox", { name: "status" });
        expect(trigger).toHaveClass("glass-select-trigger");
        expect(trigger).toHaveAttribute("aria-expanded", "false");

        await user.click(trigger);
        expect(trigger).toHaveAttribute("aria-expanded", "true");
        expect(
            screen.getByRole("listbox", { name: "status options" }),
        ).toHaveClass("glass-select-menu");
        const completed = screen.getByRole("option", { name: "Completed" });
        expect(completed).toHaveClass("glass-select-option");
        await user.hover(completed);
        expect(completed).toHaveAttribute("data-active", "true");

        await user.click(completed);
        expect(onChange).toHaveBeenCalledWith("COMPLETED");
        expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
        expect(trigger).toHaveFocus();

        await user.click(trigger);
        expect(screen.getByRole("listbox")).toBeInTheDocument();
        await user.click(document.body);
        expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    });

    it("supports arrow selection and Escape from the keyboard", async () => {
        const user = userEvent.setup();
        const onChange = vi.fn();
        render(GlassSelect, {
            id: "keyboard-status-filter",
            label: "status",
            value: "WATCHING",
            options,
            onChange,
        });

        const trigger = screen.getByRole("combobox", { name: "status" });
        trigger.focus();
        await user.keyboard("{ArrowDown}{ArrowDown}{Enter}");
        expect(onChange).toHaveBeenCalledWith("COMPLETED");
        expect(screen.queryByRole("listbox")).not.toBeInTheDocument();

        await user.keyboard("{Enter}{Escape}");
        expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
        expect(trigger).toHaveFocus();
    });
});
