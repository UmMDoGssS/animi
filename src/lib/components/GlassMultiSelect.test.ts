import { fireEvent, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";

import GlassMultiSelect from "./GlassMultiSelect.svelte";

const options = [
  { value: "Action", label: "Action" },
  { value: "Drama", label: "Drama" },
  { value: "Fantasy", label: "Fantasy" },
] as const;

it("keeps the menu open while selecting multiple options", async () => {
  const user = userEvent.setup();
  const onChange = vi.fn();
  const { rerender } = render(GlassMultiSelect, {
    id: "genre-filter",
    label: "genre",
    defaultLabel: "All genres",
    selectedValues: [],
    options,
    onChange,
  });

  const trigger = screen.getByRole("combobox", { name: "genre" });
  await user.click(trigger);

  const listbox = screen.getByRole("listbox", { name: "genre options" });
  expect(listbox).toHaveAttribute("aria-multiselectable", "true");
  await user.click(screen.getByRole("option", { name: "Drama" }));
  expect(onChange).toHaveBeenLastCalledWith(["Drama"]);
  expect(trigger).toHaveAttribute("aria-expanded", "true");

  await rerender({
    id: "genre-filter",
    label: "genre",
    defaultLabel: "All genres",
    selectedValues: ["Drama"],
    options,
    onChange,
  });
  await user.click(screen.getByRole("option", { name: "Action" }));
  expect(onChange).toHaveBeenLastCalledWith(["Action", "Drama"]);
  expect(trigger).toHaveTextContent("Drama");
});

it("supports keyboard toggling, clearing, and Escape", async () => {
  const onChange = vi.fn();
  render(GlassMultiSelect, {
    id: "tag-filter",
    label: "tag",
    defaultLabel: "All tags",
    selectedValues: ["Action", "Drama"],
    options,
    onChange,
  });

  const trigger = screen.getByRole("combobox", { name: "tag" });
  trigger.focus();
  await fireEvent.keyDown(trigger, { key: "Enter" });
  expect(trigger).toHaveAttribute("aria-expanded", "true");
  await fireEvent.keyDown(trigger, { key: "End" });
  await fireEvent.keyDown(trigger, { key: " " });
  expect(onChange).toHaveBeenLastCalledWith(["Action", "Drama", "Fantasy"]);

  await fireEvent.keyDown(trigger, { key: "Escape" });
  expect(trigger).toHaveAttribute("aria-expanded", "false");
  expect(trigger).toHaveFocus();

  await userEvent.click(trigger);
  await userEvent.click(
    screen.getByRole("button", { name: "clear tag selections" }),
  );
  expect(onChange).toHaveBeenLastCalledWith([]);
});
