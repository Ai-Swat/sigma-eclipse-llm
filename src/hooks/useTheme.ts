import { useState, useEffect } from "react";

export const useTheme = () => {
  const [theme, setTheme] = useState<"dark" | "white">("dark");

  // Initialize theme from localStorage
  useEffect(() => {
    try {
      const savedTheme = localStorage.getItem("theme");
      if (savedTheme === "dark" || savedTheme === "white") {
        setTheme(savedTheme);
        document.documentElement.className = `theme-${savedTheme}`;
      } else {
        document.documentElement.className = "theme-dark";
      }
    } catch (error) {
      console.error("Failed to load theme:", error);
      document.documentElement.className = "theme-dark";
    }
  }, []);

  // Toggle theme and save to localStorage
  const toggleTheme = (newTheme: "dark" | "white") => {
    if (newTheme === theme) return;

    setTheme(newTheme);
    document.documentElement.className = `theme-${newTheme}`;

    try {
      localStorage.setItem("theme", newTheme);
    } catch (error) {
      console.error("Failed to save theme:", error);
    }
  };

  return { theme, toggleTheme };
};
