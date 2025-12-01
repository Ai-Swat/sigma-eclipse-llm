import CheckIcon from "../../icons/check-icon.svg?react";
import styles from "./styles.module.css";

interface CheckBoxProps {
  name?: string;
  checked?: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}

export function CheckBox({ name, checked, onChange, disabled }: CheckBoxProps) {
  return (
    <div className={styles.wrapper}>
      <div className={styles.label}>
        <input
          type="checkbox"
          checked={checked}
          disabled={disabled}
          onChange={(e) => onChange(e.target.checked)}
          className={styles.input}
        />
        <span className={styles.custom}>
          <CheckIcon width={14} height={14} className={styles.check} />
        </span>
        {name && <span className={styles.text}>{name}</span>}
      </div>
    </div>
  );
}
