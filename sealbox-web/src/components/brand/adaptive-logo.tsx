import * as React from "react";
import sealboxLogo from "@/assets/sealbox-logo.svg?url";
import { cn } from "@/lib/utils";

type Props = {
  size?: number;
  className?: string;
  title?: string;
};

// Renders the SVG file and inverts it in dark mode to maintain contrast.
export function AdaptiveLogo({
  size = 28,
  className,
  title = "Sealbox",
}: Props) {
  return (
    <img
      src={sealboxLogo}
      alt={title}
      width={size}
      height={size}
      className={cn("select-none filter", className)}
    />
  );
}
