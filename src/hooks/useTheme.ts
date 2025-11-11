import { useState, useEffect } from "react";
import { Store } from "@tauri-apps/plugin-store";

export const useTheme = () => {
  const [store, setStore] = useState<Store | null>(null);
  const [theme, setTheme] = useState<"dark" | "white">("dark");

  // Initialize store and load theme
  useEffect(() => {
    const initStore = async () => {
      try {
        const loadedStore = await Store.load("settings.json");
        setStore(loadedStore);

        const savedTheme = await loadedStore.get<string>("theme");
        if (savedTheme === "dark" || savedTheme === "white") {
          setTheme(savedTheme);
          document.documentElement.className = `theme-${savedTheme}`;
        } else {
          document.documentElement.className = "theme-dark";
        }
      } catch (error) {
        console.error("Failed to initialize store or load theme:", error);
        document.documentElement.className = "theme-dark";
      }
    };
    initStore();
  }, []);

  // Toggle theme and save to store
  const toggleTheme = async () => {
    const newTheme = theme === "dark" ? "white" : "dark";
    setTheme(newTheme);
    document.documentElement.className = `theme-${newTheme}`;

    if (store) {
      try {
        await store.set("theme", newTheme);
        await store.save();
      } catch (error) {
        console.error("Failed to save theme:", error);
      }
    }
  };

  return { theme, toggleTheme };
};
