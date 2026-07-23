/*
==================================================
DANCE TRACKER 5000
RECORDING ENGINE
==================================================
*/


export class Recorder {


    constructor(canvas){

        this.canvas = canvas;

        this.recorder = null;

        this.chunks = [];

        this.recording = false;

    }




    start(){

        if(this.recording)
            return;


        let stream =
            this.canvas.captureStream(60);



        this.chunks = [];



        let options = {};



        if(
            MediaRecorder.isTypeSupported(
                "video/mp4"
            )
        ){

            options.mimeType =
                "video/mp4";

        }
        else if(
            MediaRecorder.isTypeSupported(
                "video/webm;codecs=vp9"
            )
        ){

            options.mimeType =
                "video/webm;codecs=vp9";

        }
        else if(
            MediaRecorder.isTypeSupported(
                "video/webm"
            )
        ){

            options.mimeType =
                "video/webm";

        }



        this.recorder =
            new MediaRecorder(
                stream,
                options
            );



        this.recorder.ondataavailable =
            event=>{


                if(event.data.size > 0){

                    this.chunks.push(
                        event.data
                    );

                }

            };



        this.recorder.start();



        this.recording = true;



        console.log(
            "Recording started",
            this.recorder.mimeType
        );

    }





    stop(){

        if(
            !this.recorder ||
            !this.recording
        )
            return;



        this.recorder.onstop =
            ()=>{


                let blob =
                    new Blob(
                        this.chunks,
                        {
                            type:
                            this.recorder.mimeType
                        }
                    );



                let url =
                    URL.createObjectURL(
                        blob
                    );



                let link =
                    document.createElement(
                        "a"
                    );



                link.href = url;



                let extension =
                    this.recorder.mimeType.includes(
                        "mp4"
                    )
                    ?
                    "mp4"
                    :
                    "webm";



                link.download =
                    "dance-tracker-recording."
                    +
                    extension;



                link.click();



                URL.revokeObjectURL(
                    url
                );



                console.log(
                    "Recording saved"
                );

            };



        this.recorder.stop();



        this.recording = false;



        console.log(
            "Recording stopped"
        );


        this.recorder = null;

    }


}