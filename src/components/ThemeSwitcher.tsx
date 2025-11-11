import SunIcon from "../icons/sun.svg?react";
import MoonIcon from "../icons/moon-01.svg?react";
import "./ThemeSwitcher.css";

interface ThemeSwitcherProps {
  theme: "dark" | "white";
  onToggleTheme: (newTheme: "dark" | "white") => void;
}

export const ThemeSwitcher = ({ theme, onToggleTheme }: ThemeSwitcherProps) => {
  return (
    <div className="theme-switcher">
      <div
        className={`active-background ${theme === "white" ? "left-position" : "right-position"}`}
      >
        <span></span>
      </div>

      <button className={theme === "white" ? "active" : ""} onClick={() => onToggleTheme("white")}>
        <SunIcon width={18} height={18} />
      </button>

      <button className={theme === "dark" ? "active" : ""} onClick={() => onToggleTheme("dark")}>
        <MoonIcon width={18} height={18} />
      </button>
    </div>
  );
};
