# Sealbox Logo Setup Instructions

## Steps to Add Your Seal Logo

1. **Prepare your logo files** in the following formats:
   - Primary: `sealbox-logo.svg` (vector format, recommended)
   - Fallback: `sealbox-logo.png` (high resolution, 512x512px or larger)
   - Optional variants:
     - `sealbox-logo-white.svg` (for dark backgrounds)
     - `sealbox-logo-dark.svg` (for light backgrounds)
     - `sealbox-icon.svg` (icon-only version without text)

2. **Place the files** in this directory (`src/assets/images/logos/`)

3. **Update the SealboxLogo component** (`src/components/ui/sealbox-logo.tsx`):

   Replace the fallback `SealIcon` component with actual logo imports:

   ```typescript
   // Add at the top of the file
   import sealboxLogo from "@/assets/images/logos/sealbox-logo.svg";
   import sealboxLogoWhite from "@/assets/images/logos/sealbox-logo-white.svg";
   import sealboxLogoDark from "@/assets/images/logos/sealbox-logo-dark.svg";

   // Replace the LogoImage component
   const LogoImage = () => {
     const logoSrc = variant === "white" ? sealboxLogoWhite :
                    variant === "dark" ? sealboxLogoDark :
                    sealboxLogo;

     return (
       <img
         src={logoSrc}
         alt="Sealbox"
         className={cn(getSizeClasses(), className)}
       />
     );
   };
   ```

4. **Add Vite configuration** (if needed) to handle SVG imports in `vite.config.ts`:

   ```typescript
   import { defineConfig } from 'vite'
   import react from '@vitejs/plugin-react'

   export default defineConfig({
     plugins: [react()],
     assetsInclude: ['**/*.svg']
   })
   ```

## Current Integration Status

✅ Logo structure created  
✅ SealboxLogo component implemented  
✅ Login page updated to use seal logo  
✅ Navigation bar updated to use seal logo  
✅ Fallback seal icon implemented (temporary)

## Ready for Your Logo!

The component system is ready. Just add your actual logo files and update the component as described above.