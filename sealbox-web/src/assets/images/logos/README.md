# Sealbox Logo Assets

This directory contains the official Sealbox logo files.

## Logo Variants

Please place your logo files here with the following naming convention:

- `sealbox-logo.svg` - Main vector logo (recommended)
- `sealbox-logo.png` - High-resolution PNG (for fallback)
- `sealbox-logo-small.svg` - Small/icon version
- `sealbox-logo-white.svg` - White version for dark backgrounds
- `sealbox-logo-dark.svg` - Dark version for light backgrounds

## Usage Guidelines

- Use SVG format when possible for scalability
- Maintain aspect ratio when resizing
- Ensure adequate contrast against background
- Use appropriate variant based on background color

## Integration

The logo is integrated through the `SealboxLogo` component located at:
`src/components/ui/sealbox-logo.tsx`

This component automatically selects the appropriate logo variant based on the current theme and size requirements.