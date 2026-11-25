// Minimalist MyCode emblem icon.
export function MyAppIcon(props: { readonly className?: string }) {
  return (
    <svg
      viewBox="0 0 64 64"
      xmlns="http://www.w3.org/2000/svg"
      aria-hidden="true"
      className={props.className}
    >
      <rect width="64" height="64" rx="14" className="fill-foreground" />
      <path
        d="M14 44V20H21L29 31L37 20H44V44H37V31L29 41L21 31V44H14ZM50 44H46V20H50V44Z"
        className="fill-background"
      />
    </svg>
  );
}
