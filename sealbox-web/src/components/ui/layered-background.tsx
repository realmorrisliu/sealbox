import { cn } from "@/lib/utils";

interface LayeredBackgroundProps {
  variant?: "subtle" | "rich" | "minimal";
  texture?: "dots" | "grid" | "squares" | "hexagon" | "diagonal" | "none";
  animated?: boolean;
  children?: React.ReactNode;
  className?: string;
}

export function LayeredBackground({
  variant = "subtle",
  texture = "dots",
  animated = false,
  children,
  className,
}: LayeredBackgroundProps) {
  const getTextureClass = () => {
    switch (texture) {
      case "dots":
        return "bg-texture-dots-subtle";
      case "grid":
        return "bg-texture-grid-fine";
      case "squares":
        return "bg-texture-squares";
      case "hexagon":
        return "bg-texture-hexagon";
      case "diagonal":
        return "bg-texture-diagonal";
      default:
        return "";
    }
  };

  const getVariantClass = () => {
    switch (variant) {
      case "rich":
        return "bg-layered-rich";
      case "minimal":
        return "bg-layered-base";
      default:
        return "bg-layered-subtle";
    }
  };

  return (
    <div className={cn("relative min-h-screen overflow-hidden", className)}>
      {/* Main background layer */}
      <div className={cn("absolute inset-0", getVariantClass())} />
      
      {/* Texture layer */}
      {texture !== "none" && (
        <div className={cn("absolute inset-0", getTextureClass())} />
      )}
      
      {/* Animated decorative elements */}
      {animated && (
        <>
          <div className="absolute -top-40 -right-40 w-96 h-96 bg-gradient-to-br from-primary/8 to-transparent rounded-full blur-3xl bg-animate-float" />
          <div className="absolute -bottom-40 -left-40 w-96 h-96 bg-gradient-to-br from-accent/8 to-transparent rounded-full blur-3xl bg-animate-float" style={{ animationDelay: "2s" }} />
          <div className="absolute top-1/3 left-1/2 w-64 h-64 bg-gradient-to-br from-secondary/6 to-transparent rounded-full blur-2xl bg-animate-float" style={{ animationDelay: "4s" }} />
        </>
      )}
      
      {/* Content layer */}
      <div className="relative z-10">
        {children}
      </div>
    </div>
  );
}

interface BackgroundPatternProps {
  pattern: "circuit" | "waves" | "geometric";
  opacity?: number;
  className?: string;
}

export function BackgroundPattern({ 
  pattern, 
  opacity = 0.1, 
  className 
}: BackgroundPatternProps) {
  const getPatternSVG = () => {
    switch (pattern) {
      case "circuit":
        return (
          <svg width="60" height="60" viewBox="0 0 60 60" xmlns="http://www.w3.org/2000/svg">
            <g fill="none" stroke="currentColor" strokeWidth="1" opacity={opacity}>
              <path d="M10 10h40v40H10z"/>
              <circle cx="20" cy="20" r="2"/>
              <circle cx="40" cy="20" r="2"/>
              <circle cx="20" cy="40" r="2"/>
              <circle cx="40" cy="40" r="2"/>
              <path d="M20 18v-8h20v8M40 22v8h-20v-8"/>
            </g>
          </svg>
        );
      case "waves":
        return (
          <svg width="60" height="20" viewBox="0 0 60 20" xmlns="http://www.w3.org/2000/svg">
            <path 
              d="M0,10 Q15,0 30,10 T60,10" 
              fill="none" 
              stroke="currentColor" 
              strokeWidth="0.5" 
              opacity={opacity}
            />
          </svg>
        );
      case "geometric":
        return (
          <svg width="40" height="40" viewBox="0 0 40 40" xmlns="http://www.w3.org/2000/svg">
            <g fill="currentColor" opacity={opacity}>
              <circle cx="20" cy="20" r="1"/>
              <circle cx="10" cy="10" r="0.5"/>
              <circle cx="30" cy="10" r="0.5"/>
              <circle cx="10" cy="30" r="0.5"/>
              <circle cx="30" cy="30" r="0.5"/>
            </g>
          </svg>
        );
      default:
        return null;
    }
  };

  return (
    <div 
      className={cn("absolute inset-0 opacity-50", className)}
      style={{
        backgroundImage: `url("data:image/svg+xml,${encodeURIComponent(getPatternSVG()?.outerHTML || '')}")`,
        backgroundRepeat: "repeat",
        backgroundSize: pattern === "waves" ? "60px 20px" : "40px 40px"
      }}
    />
  );
}