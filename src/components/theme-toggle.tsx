import { MonitorIcon, MoonIcon, SunIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { type Theme, useThemeStore } from "@/stores/theme";

const order: Theme[] = ["system", "light", "dark"];
const icons = { system: MonitorIcon, light: SunIcon, dark: MoonIcon };
const labels = { system: "System theme", light: "Light theme", dark: "Dark theme" };

export function ThemeToggle() {
  const theme = useThemeStore((s) => s.theme);
  const setTheme = useThemeStore((s) => s.setTheme);
  const Icon = icons[theme];
  const next = order[(order.indexOf(theme) + 1) % order.length];

  return (
    <Tooltip>
      <TooltipTrigger
        render={
          <Button
            variant="ghost"
            size="icon"
            aria-label={labels[theme]}
            onClick={() => setTheme(next)}
          />
        }
      >
        <Icon />
      </TooltipTrigger>
      <TooltipContent>{labels[theme]}</TooltipContent>
    </Tooltip>
  );
}
