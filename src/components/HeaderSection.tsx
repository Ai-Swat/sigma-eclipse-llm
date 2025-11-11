import logo from "../assets/logo2.png";

interface HeaderSectionProps {
  theme: "dark" | "white";
  isSettingsOpen: boolean;
  onToggleTheme: () => void;
  onToggleSettings: () => void;
}

export const HeaderSection = ({ 
  theme, 
  isSettingsOpen, 
  onToggleTheme, 
  onToggleSettings 
}: HeaderSectionProps) => {
  console.log(isSettingsOpen);
  return (
    <div className="header-section">
      <h1>
        <img src={logo} alt="Shield" className="logo-icon" /> 
        Sigma Shield LLM
      </h1>
      <div className="theme-toggle-container">
        <button 
          className="theme-toggle settings-button"
          onClick={onToggleSettings}
          title="Settings"
        >
          <span className="settings-button-icon">⚙</span>
        </button>
        <button 
          className="theme-toggle theme-toggle-icon" 
          onClick={onToggleTheme} 
          title="Toggle theme"
        >
          {theme === "dark" ? "○" : "●"}
        </button>
      </div>
    </div>
  );
};

