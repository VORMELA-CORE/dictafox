import appIcon from "../assets/brand/dictafox-app-icon.svg";
import lockupDark from "../assets/brand/vormela-core-lockup-dark.svg";
import lockupLight from "../assets/brand/vormela-core-lockup-light.svg";
import { api } from "../services/api";

/** The DictaFox app icon: the Vormela Core mark on night, as the brand guide specifies. */
export function AppIcon({ size = 32, label = "" }: { size?: number; label?: string }) {
  return (
    <img
      src={appIcon}
      width={size}
      height={size}
      alt={label}
      className="app-icon"
      draggable={false}
    />
  );
}

/** The official Vormela Core horizontal lockup, in the colourway for the active theme. */
export function VormelaLockup({ height = 26 }: { height?: number }) {
  return (
    <span className="vc-lockup" role="img" aria-label="Vormela Core">
      <img className="vc-lockup-dark" src={lockupDark} height={height} alt="" draggable={false} />
      <img className="vc-lockup-light" src={lockupLight} height={height} alt="" draggable={false} />
    </span>
  );
}

/** Opens one of the fixed Vormela Core links in the default browser. */
export function VormelaLink({ link, children }: { link: "website" | "repository"; children: string }) {
  return (
    <button
      type="button"
      className="link-button"
      onClick={() => void api.openLink(link).catch(() => undefined)}
    >
      {children}
    </button>
  );
}

/** "A product of Vormela Core" credit line, shown wherever the product introduces itself. */
export function ProductCredit({ lockupHeight = 22 }: { lockupHeight?: number }) {
  return (
    <p className="product-credit">
      <span>A product of</span>
      <VormelaLockup height={lockupHeight} />
    </p>
  );
}
