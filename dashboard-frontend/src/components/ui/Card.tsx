import React from "react";

export function Card({ className = "", children }: { className?: string; children: React.ReactNode }) {
  return (
    <div className={"card relative overflow-hidden " + className}>
      <div className="pointer-events-none absolute inset-0 bg-white/2 [mask-image:radial-gradient(60%_60%_at_30%_0%,black,transparent)]" />
      <div className="absolute inset-0 border border-white/5 rounded-[inherit]" />
      <div className="relative p-6">{children}</div>
    </div>
  );
}

export function CardHeader({ icon, title, actions }: { icon?: React.ReactNode; title: string; actions?: React.ReactNode }) {
  return (
    <div className="mb-4 flex items-center justify-between">
      <div className="flex items-center gap-2">
        {icon}
        <h2 className="text-lg font-medium">{title}</h2>
      </div>
      <div className="flex items-center gap-2">{actions}</div>
    </div>
  );
}

export function CardContent({ children, className = "" }: { children: React.ReactNode; className?: string }) {
  return <div className={className}>{children}</div>;
}

