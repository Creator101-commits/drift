import React from 'react';

interface DriftLogoProps {
  size?: number;
  className?: string;
}

export const DriftLogo: React.FC<DriftLogoProps> = ({ size = 22, className }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 100 100"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    className={className}
    style={{ flexShrink: 0, display: 'inline-block', verticalAlign: 'middle' }}
  >
    <rect width="100" height="100" rx="22" fill="#181818" />
    <path d="M50 18L78 76L50 64L22 76Z" fill="#ffffff" />
    <circle cx="50" cy="46" r="5.5" fill="#181818" />
  </svg>
);

export default DriftLogo;
