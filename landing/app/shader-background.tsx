"use client";

import { MeshGradient } from "@paper-design/shaders-react";

/**
 * Subtle, brand-accurate background: a slow charcoal mesh gradient with a
 * single vermilion accent spot. Kept deliberately understated and darkened by
 * an overlay so the foreground text stays crisp and the page reads as minimal.
 */
export function ShaderBackground() {
  return (
    <div aria-hidden className="pointer-events-none fixed inset-0 -z-10">
      <MeshGradient
        className="h-full w-full"
        colors={["#151515", "#151515", "#FA4616", "#1b1512"]}
        distortion={0.8}
        swirl={0.05}
        speed={0.25}
        scale={1.1}
        offsetY={-0.35}
      />
      {/* Darkening + vignette overlay to keep it subtle and legible. */}
      <div
        className="absolute inset-0"
        style={{
          background:
            "radial-gradient(120% 90% at 50% 0%, rgba(21,21,21,0) 0%, rgba(21,21,21,0.55) 55%, #151515 100%)",
        }}
      />
    </div>
  );
}
