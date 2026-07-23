/*
==================================================
DANCE TRACKER 5000
CAMERA ENGINE
==================================================
*/


export class Camera {



    constructor(){


        this.video =

        document.getElementById(

            "camera"

        );



        this.stream=null;



        this.enabled=false;



    }








    async start(){



        try {



            this.stream =

            await navigator.mediaDevices
            .getUserMedia({



                video:{



                    width:320,

                    height:240



                },



                audio:false



            });





            this.video.srcObject=

            this.stream;



            this.enabled=true;



            console.log(

                "Camera started"

            );



        }



        catch(error){



            console.error(

                "Camera error:",

                error

            );



        }



    }








    stop(){



        if(this.stream){



            this.stream

            .getTracks()

            .forEach(track=>{


                track.stop();


            });



        }



        this.enabled=false;



    }







    getVideo(){



        return this.video;



    }





}