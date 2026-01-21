import type { SVGProps } from 'react'

import { cn } from '@/lib/utils'

export default function Check({ className, ...props }: SVGProps<SVGSVGElement>) {
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
      <path d={'M20 6 9 17l-5-5'} />
    </svg>
  )
}
