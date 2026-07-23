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





        this.chunks=[];




        this.recorder =

        new MediaRecorder(

            stream,

            {

                mimeType:

                "video/webm"

            }

        );







        this.recorder.ondataavailable =

        event=>{


            if(event.data.size>0){


                this.chunks.push(

                    event.data

                );


            }


        };







        this.recorder.start();



        this.recording=true;



        console.log(

            "Recording started"

        );



    }







    stop(){



        if(!this.recorder ||

           !this.recording)

           return;





        this.recorder.stop();




        this.recorder.onstop=()=>{



            let blob =

            new Blob(

                this.chunks,

                {

                    type:"video/webm"

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



            link.href=url;


            link.download=

            "dance-tracker-recording.webm";



            link.click();



            URL.revokeObjectURL(

                url

            );



        };




        this.recording=false;



        console.log(

            "Recording stopped"

        );



    }





}