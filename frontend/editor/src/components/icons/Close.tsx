import type { SVGProps } from 'react'

import { cn } from '@/lib/utils'

export default function Close({ className, ...props }: SVGProps<SVGSVGElement>) {
  return (
    <svg
      className={cn('size-6', className)}
      xmlns={'http://www.w3.org/2000/svg'}
      viewBox={'0 0 24 24'}
      fill={'none'}
      stroke={'currentColor'}
      strokeWidth={'2'}
      strokeLinecap={'round'}
      strokeLinejoin={'round'}
      {...props}
    >
      <path d={'M18 6 6 18'} />
      <path d={'m6 6 12 12'} />
    </svg>
  )
}
