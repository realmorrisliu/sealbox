import { cn } from "@/lib/utils";
import sealboxLogo from "@/assets/images/logos/sealbox-logo.svg";

interface SealboxLogoProps {
  size?: "sm" | "md" | "lg" | "xl";
  variant?: "default" | "white" | "dark";
  className?: string;
  showText?: boolean;
}

export function SealboxLogo({ 
  size = "md", 
  variant = "default", 
  className,
  showText = true 
}: SealboxLogoProps) {
  const getSizeClasses = () => {
    switch (size) {
      case "sm":
        return showText ? "h-6 w-auto" : "h-6 w-6";
      case "md":
        return showText ? "h-8 w-auto" : "h-8 w-8";
      case "lg":
        return showText ? "h-10 w-auto" : "h-10 w-10";
      case "xl":
        return showText ? "h-12 w-auto" : "h-12 w-12";
      default:
        return showText ? "h-8 w-auto" : "h-8 w-8";
    }
  };

  // Actual logo image component
  const LogoImage = () => (
    <img
      src={sealboxLogo}
      alt="Sealbox"
      className={cn(getSizeClasses(), "object-contain", className)}
    />
  );

  if (!showText) {
    return <LogoImage />;
  }

  return (
    <div className={cn("flex items-center space-x-2", className)}>
      <LogoImage />
      {showText && (
        <span className={cn(
          "font-semibold tracking-tight",
          size === "sm" && "text-sm",
          size === "md" && "text-base",
          size === "lg" && "text-lg",
          size === "xl" && "text-xl",
          variant === "white" && "text-white",
          variant === "dark" && "text-foreground",
          variant === "default" && "text-gradient"
        )}>
          Sealbox
        </span>
      )}
    </div>
  );
}

// Alternative export for when you only need the icon
export function SealboxIcon({ 
  size = "md", 
  className 
}: Pick<SealboxLogoProps, "size" | "className">) {
  return <SealboxLogo size={size} className={className} showText={false} />;
}