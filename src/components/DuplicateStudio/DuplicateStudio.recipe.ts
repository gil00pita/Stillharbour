import { defineRecipe } from '@chakra-ui/react';

export const duplicateStudioRecipe = defineRecipe({
  base: {
    '&': {
      marginBlock: '24px',
      minWidth: '0',
    },
    '& p': { color: 'fg.muted' },
    '& .duplicate-summary, & .duplicate-summary-actions, & .duplicate-mode-bar, & .duplicate-detail-head, & .duplicate-rule-actions':
      {
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        gap: '12px',
        flexWrap: 'wrap',
      },
    '& .duplicate-summary h2': {
      margin: '0 0 8px',
      fontSize: '24px',
      fontWeight: '500',
      letterSpacing: '-0.02em',
    },
    '& .duplicate-summary p': {
      margin: '0',
      maxWidth: '70ch',
    },
    '& .duplicate-summary-actions': {
      justifyContent: 'flex-end',
    },
    '& .duplicate-totals': {
      display: 'grid',
      gridTemplateColumns: 'repeat(4, minmax(0, 1fr))',
      margin: '24px 0 16px',
      paddingBlock: '16px',
      borderBlock: '1px solid var(--border)',
      gap: '16px',
    },
    '& .duplicate-totals dt, & .duplicate-detail-stats dt': {
      fontSize: '12px',
      color: 'var(--muted)',
    },
    '& .duplicate-totals dd': {
      margin: '4px 0 0',
      fontSize: '22px',
      fontWeight: '500',
    },
    '& .duplicate-totals > div:last-child dd': {
      fontSize: '16px',
      paddingTop: '6px',
    },
    '& .duplicate-safety-note': {
      fontSize: '12px',
      maxWidth: '100ch',
      marginBottom: '24px',
    },
    '& .duplicate-workspace': {
      display: 'grid',
      gridTemplateColumns: 'minmax(240px, 0.8fr) minmax(0, 2fr)',
      minHeight: '440px',
      alignItems: 'start',
      borderTop: '1px solid var(--border)',
    },
    '& .duplicate-queue': {
      minWidth: '0',
      padding: '20px 20px 20px 0',
    },
    '& .duplicate-queue h3': {
      margin: '0 0 16px',
      fontWeight: '500',
    },
    '& .duplicate-queue-head': {
      marginBottom: '16px',
    },
    '& .duplicate-filters': {
      display: 'flex',
      flexWrap: 'wrap',
      gap: '4px',
      marginTop: '12px',
    },
    '& .duplicate-filters button': {
      padding: '4px 8px',
      minHeight: '30px',
      fontSize: '12px',
    },
    "& .duplicate-filters button[aria-pressed='true'], & .duplicate-mode-bar button[aria-pressed='true']":
      {
        background: 'var(--stillharbour-colors-brand-selected)',
        color: 'var(--accent)',
      },
    '& .duplicate-select-all': {
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between',
      gap: '8px',
      fontSize: '12px',
      color: 'var(--muted)',
    },
    '& .duplicate-select-all label': {
      display: 'flex',
      alignItems: 'center',
      gap: '8px',
      margin: '8px 0',
    },
    '& .duplicate-clusters': {
      maxHeight: '600px',
      overflow: 'auto',
      scrollbarColor: 'var(--border) var(--background)',
    },
    '& .duplicate-cluster': {
      display: 'flex',
      gap: '4px',
      alignItems: 'center',
      borderBottom: '1px solid var(--border)',
      paddingBlock: '4px',
    },
    '& .duplicate-cluster.is-current': {
      background: 'var(--surface)',
      borderRadius: 'var(--control-radius)',
    },
    '& .duplicate-cluster-select': {
      flexShrink: '0',
      paddingLeft: '8px',
    },
    '& .duplicate-cluster > button': {
      display: 'flex',
      gap: '12px',
      padding: '12px 8px',
      minWidth: '0',
      flex: '1',
      textAlign: 'left',
      whiteSpace: 'normal',
      background: 'transparent',
      alignItems: 'flex-start',
    },
    '& .duplicate-cluster > button > span:last-child': {
      minWidth: '0',
    },
    '& .duplicate-cluster strong, & .duplicate-cluster span span, & .duplicate-cluster small': {
      display: 'block',
      overflowWrap: 'anywhere',
    },
    '& .duplicate-cluster strong': {
      fontSize: '13px',
      fontWeight: '500',
      marginBottom: '4px',
    },
    '& .duplicate-cluster small, & .duplicate-cluster span span': {
      fontSize: '12px',
      color: 'var(--muted)',
    },
    '& .duplicate-cluster small': {
      marginTop: '4px',
    },
    '& .duplicate-cluster-number': {
      color: 'var(--muted)',
      fontSize: '12px',
      paddingTop: '1px',
    },
    '& .duplicate-inspector': {
      minWidth: '0',
      padding: '24px',
      borderLeft: '1px solid var(--border)',
    },
    '& .duplicate-detail-head': {
      alignItems: 'flex-start',
    },
    '& .duplicate-detail-head h3': {
      fontSize: '18px',
      fontWeight: '500',
      margin: '0 0 6px',
    },
    '& .duplicate-detail-head p': {
      fontSize: '13px',
      margin: '0',
    },
    '& .duplicate-evidence-badge': {
      color: 'var(--accent)',
      background: 'var(--surface)',
      fontSize: '12px',
      padding: '6px 8px',
      borderRadius: '4px',
    },
    '& .duplicate-detail-stats': {
      display: 'flex',
      flexWrap: 'wrap',
      gap: '16px 32px',
      paddingBlock: '16px',
      margin: '0',
    },
    '& .duplicate-detail-stats dd': {
      margin: '4px 0 0',
      fontSize: '13px',
    },
    '& [data-scope=field] > label:not(.screen-reader-status)': {
      display: 'block',
      marginBottom: '8px',
      fontSize: '13px',
    },
    '& .duplicate-pair-controls': {
      display: 'grid',
      gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
      gap: '12px',
      marginBlock: '8px 16px',
    },
    '& .duplicate-mode-bar': {
      justifyContent: 'flex-start',
      gap: '4px',
    },
    '& .duplicate-mode-bar button': {
      padding: '4px 8px',
      minHeight: '32px',
      fontSize: '12px',
    },
    '& .duplicate-preview-note': {
      fontSize: '12px',
      marginBlock: '8px 12px',
      maxWidth: '75ch',
    },
    '& .duplicate-stage': {
      position: 'relative',
      minHeight: '240px',
      height: 'clamp(240px, 32vw, 420px)',
      background: 'var(--surface)',
      overflow: 'hidden',
      borderRadius: '8px',
      isolation: 'isolate',
    },
    '& .duplicate-stage figure': {
      minWidth: '0',
      margin: '0',
      position: 'relative',
      width: '100%',
      height: '100%',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
    },
    '& .duplicate-stage img': {
      width: '100%',
      height: '100%',
      objectFit: 'contain',
    },
    '& .duplicate-stage figcaption': {
      position: 'absolute',
      bottom: '8px',
      left: '8px',
      maxWidth: 'calc(100% - 16px)',
      background: 'var(--background)',
      color: 'var(--secondary)',
      padding: '4px 8px',
      fontSize: '12px',
      borderRadius: '4px',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap',
    },
    '& .duplicate-stage-pair': {
      display: 'grid',
      gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
      gap: '4px',
    },
    '& .duplicate-stage-overlay figure': {
      position: 'absolute',
      inset: '0',
    },
    '& .duplicate-stage-overlay figure:nth-child(2) figcaption': {
      left: 'auto',
      right: '8px',
      bottom: '40px',
    },
    '& .duplicate-slider-line': {
      position: 'absolute',
      top: '0',
      bottom: '0',
      width: '2px',
      background: 'var(--accent)',
      pointerEvents: 'none',
    },
    '& .duplicate-slider-line::after': {
      content: "''",
      position: 'absolute',
      top: '50%',
      left: '-5px',
      width: '12px',
      height: '32px',
      borderRadius: '4px',
      background: 'var(--accent)',
    },
    '& .duplicate-stage-overlay .duplicate-heat-layer:nth-child(2)': {
      mixBlendMode: 'difference',
    },
    '& .duplicate-heat-layer:first-child': { backgroundColor: 'black' },
    '& .duplicate-heat-layer figcaption': {
      display: 'none',
    },
    '& .duplicate-image-state': {
      color: 'var(--muted)',
      padding: '16px',
      fontSize: '12px',
    },
    "& .duplicate-comparison input[type='range'], & .duplicate-dialog-body input[type='range']": {
      width: '100%',
      accentColor: 'var(--accent)',
      padding: '0',
    },
    '& .duplicate-signals': {
      marginTop: '24px',
    },
    '& .duplicate-signals h4, & .duplicate-member-review h4': {
      fontWeight: '500',
      fontSize: '14px',
      marginBottom: '8px',
    },
    '& .duplicate-signals > p': {
      marginTop: '8px',
      fontSize: '13px',
      maxWidth: '75ch',
    },
    '& .duplicate-signals dl': {
      display: 'grid',
      gap: '12px',
    },
    '& .duplicate-signals dl > div': {
      display: 'grid',
      gridTemplateColumns: 'minmax(110px, 0.7fr) minmax(0, 2fr)',
      gap: '12px',
      fontSize: '12px',
    },
    '& .duplicate-signals dt': {
      color: 'var(--secondary)',
    },
    '& .duplicate-signals dd': {
      color: 'var(--muted)',
      margin: '0',
      overflowWrap: 'anywhere',
    },
    '& .duplicate-member-review': {
      borderTop: '1px solid var(--border)',
      marginTop: '24px',
      paddingTop: '8px',
    },
    '& .duplicate-rule-actions': {
      justifyContent: 'flex-start',
      alignItems: 'flex-end',
    },
    '& .duplicate-rule-actions > div': {
      flex: '1',
      minWidth: '180px',
    },
    '& .duplicate-member-review p': {
      fontSize: '12px',
    },
    '& .duplicate-member-list > label': {
      display: 'flex',
      gap: '12px',
      alignItems: 'center',
      paddingBlock: '12px',
      borderBottom: '1px solid var(--border)',
    },
    '& .duplicate-member-list > label > span:nth-child(2)': {
      flex: '1',
      minWidth: '0',
    },
    '& .duplicate-member-list strong, & .duplicate-member-list small': {
      display: 'block',
      overflowWrap: 'anywhere',
    },
    '& .duplicate-member-list small': {
      fontSize: '12px',
      color: 'var(--muted)',
      marginTop: '4px',
    },
    '& .duplicate-member-list > label > span:last-child': {
      color: 'var(--accent)',
      fontSize: '12px',
    },
    '& .duplicate-backend-review': {
      marginTop: '20px',
      fontSize: '13px',
    },
    '& .duplicate-backend-review summary, & .duplicate-dialog-body summary': {
      cursor: 'pointer',
      color: 'var(--secondary)',
    },
    '& .duplicate-empty': {
      padding: '48px 12px',
      color: 'var(--muted)',
    },
    '& .duplicate-empty h3': {
      color: 'var(--secondary)',
      fontWeight: '500',
      marginBottom: '8px',
    },
    '& .duplicate-empty p': {
      fontSize: '13px',
      maxWidth: '55ch',
    },
    '& .duplicate-review-dialog': {
      position: 'fixed',
      inset: '0 0 0 auto',
      margin: '0',
      height: '100dvh',
      maxHeight: '100dvh',
      width: 'min(460px, calc(100vw - 32px))',
      maxWidth: 'none',
      boxSizing: 'border-box',
      border: '0',
      borderLeft: '1px solid var(--border)',
      padding: '0',
      background: 'var(--surface)',
      color: 'var(--secondary)',
    },
    '& .duplicate-review-dialog[open]': {
      display: 'flex',
      flexDirection: 'column',
    },
    '& .duplicate-review-dialog::backdrop': {
      background: 'rgb(0 0 0 / 65%)',
    },
    '& .duplicate-review-dialog > header': {
      display: 'flex',
      justifyContent: 'space-between',
      alignItems: 'center',
      gap: '16px',
      padding: '20px 24px',
      borderBottom: '1px solid var(--border)',
    },
    '& .duplicate-review-dialog > header h2': {
      margin: '0',
      fontSize: '20px',
      fontWeight: '500',
      color: 'var(--stillharbour-colors-fg)',
    },
    '& .duplicate-dialog-body': {
      overflow: 'auto',
      padding: '8px 24px 32px',
      fontSize: '13px',
    },
    '& .duplicate-dialog-body p': {
      color: 'var(--muted)',
    },
    '& .duplicate-dialog-body > div, & .duplicate-dialog-body > fieldset': {
      marginBlock: '20px',
    },
    "& .duplicate-dialog-body label:has(input[type='checkbox'])": {
      display: 'flex',
      alignItems: 'center',
      gap: '8px',
      margin: '12px 0',
    },
    '& .duplicate-dialog-body fieldset': {
      border: '0',
      padding: '0',
    },
    '& .duplicate-rule-preview, & .duplicate-cleanup-confirmation': {
      borderBlock: '1px solid var(--border)',
      paddingBlock: '16px',
      marginTop: '20px',
    },
    '@media (max-width: 1100px)': {
      '& .duplicate-workspace': {
        gridTemplateColumns: 'minmax(200px, 0.8fr) minmax(0, 1.6fr)',
      },
      '& .duplicate-inspector': {
        padding: '20px 0 20px 20px',
      },
    },
    '@media (max-width: 850px)': {
      '& .duplicate-workspace': {
        display: 'block',
      },
      '& .duplicate-queue': {
        paddingRight: '0',
      },
      '& .duplicate-clusters': {
        maxHeight: '260px',
      },
      '& .duplicate-inspector': {
        padding: '24px 0',
        borderLeft: '0',
        borderTop: '1px solid var(--border)',
      },
      '& .duplicate-totals': {
        gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
      },
      '& .duplicate-summary h2': {
        fontSize: '22px',
      },
    },
  },
});
