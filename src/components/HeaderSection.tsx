import logo from "../assets/logo2.png";
import SettingsIcon from "../icons/settings-01.svg?react";
import "./HeaderSection.css";

interface HeaderSectionProps {
  onToggleSettings: () => void;
}

export const HeaderSection = ({ onToggleSettings }: HeaderSectionProps) => {
  return (
    <div className="header-section">
      <h1>
        <img src={logo} alt="Eclipse" className="logo-icon" />
        Sigma Eclipse LLM
      </h1>
      <div className="header-buttons-wrapper">
        <button className="transparent-hover-button" onClick={onToggleSettings} title="Settings">
          <span className="settings-button-icon">
            <SettingsIcon width={20} height={20} />
          </span>
        </button>
      </div>
    </div>
  );
};
