/*
==================================================
DANCE TRACKER 5000
ASPECT RATIO FIT
==================================================
*/


export function containFit(sourceWidth, sourceHeight, targetWidth, targetHeight){


    if(!sourceWidth || !sourceHeight){

        return {
            x:0,
            y:0,
            width:targetWidth,
            height:targetHeight
        };

    }


    const scale =
        Math.min(
            targetWidth / sourceWidth,
            targetHeight / sourceHeight
        );


    const width =
        sourceWidth * scale;


    const height =
        sourceHeight * scale;


    return {
        x:(targetWidth - width) / 2,
        y:(targetHeight - height) / 2,
        width:width,
        height:height
    };


}
