import type { SVGProps } from "react";

export function MyWordmark(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...props} viewBox="15 37 98 56" xmlns="http://www.w3.org/2000/svg" aria-label="MY">
      <path
        d="M15 93V37H26.5L39.5 61.5L52.5 37H64V93H53V57L42.5 77.5H36.5L26 57V93H15ZM85.2 93V68.5L69 37H81.5L91 55.5L100.5 37H113L96.8 68.5V93H85.2Z"
        fill="currentColor"
      />
    </svg>
  );
}
