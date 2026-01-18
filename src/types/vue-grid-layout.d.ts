declare module 'vue-grid-layout' {
    import { DefineComponent } from 'vue';

    interface LayoutItem {
        i: string;
        x: number;
        y: number;
        w: number;
        h: number;
        minW?: number;
        minH?: number;
        maxW?: number;
        maxH?: number;
        isDraggable?: boolean;
        isResizable?: boolean;
        static?: boolean;
        [key: string]: any;
    }

    export const GridLayout: DefineComponent<{
        layout?: LayoutItem[];
        colNum?: number;
        rowHeight?: number;
        maxRows?: number;
        margin?: [number, number];
        isDraggable?: boolean;
        isResizable?: boolean;
        isMirrored?: boolean;
        autoSize?: boolean;
        verticalCompact?: boolean;
        useCssTransforms?: boolean;
        responsive?: boolean;
        breakpoints?: Record<string, number>;
        cols?: Record<string, number>;
    }>;

    export const GridItem: DefineComponent<{
        i: string;
        x: number;
        y: number;
        w: number;
        h: number;
        minW?: number;
        minH?: number;
        maxW?: number;
        maxH?: number;
        isDraggable?: boolean;
        isResizable?: boolean;
        static?: boolean;
        dragAllowFrom?: string;
        dragIgnoreFrom?: string;
    }>;
}
