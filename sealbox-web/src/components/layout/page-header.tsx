"use client";

import { ReactNode } from "react";

export function PageHeader({
  title,
  subtitle,
  meta,
  actions,
  children,
}: {
  title: string;
  subtitle?: string;
  meta?: ReactNode;
  actions?: ReactNode;
  children?: ReactNode;
}) {
  return (
    <div className="space-y-6">
      <div className="flex items-start justify-between">
        <div className="space-y-1">
          <h1 className="text-3xl md:text-4xl font-semibold leading-tight">
            {title}
          </h1>
          {subtitle && (
            <p className="text-base text-muted-foreground">{subtitle}</p>
          )}
          {meta && <div className="pt-1">{meta}</div>}
        </div>
        {actions && <div className="flex items-center gap-3">{actions}</div>}
      </div>
      {children}
    </div>
  );
}
